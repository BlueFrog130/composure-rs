//! Commands only a hosted server can run: its filesystem, its process, and threads to block on.
//! They take `Ctx<Host<Server>>`: the [`Server`] given to `with_data` is the State's data.

use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use composure::{
    Ctx, autocomplete, command,
    command::{Deferred, Ephemeral, HandlerError},
    rest::EditMessage,
    types::{File, InteractionResponse},
};
use composure_adapter_tokio::Host;
use composure_example_bot::Config;

/// What the commands read besides the Interaction, handed to `TokioInteractionBot::with_data`:
/// the Bot Data on tokio
pub struct Server {
    /// The shared commands' settings (`/greet`)
    pub config: Config,
    /// When the process started, for `/uptime`
    pub started: Instant,
    /// The only directory `/file` reads from
    pub files_dir: PathBuf,
}

impl Server {
    /// Started now, serving `files_dir`
    pub fn new(config: Config, files_dir: impl Into<PathBuf>) -> Self {
        Self {
            config,
            started: Instant::now(),
            files_dir: files_dir.into(),
        }
    }
}

/// The shared commands take `Ctx<impl HasData<Data: AsRef<Config>>>`: this serves them from the
/// server's data
impl AsRef<Config> for Server {
    fn as_ref(&self) -> &Config {
        &self.config
    }
}

/// Longest file `/file` sends; bigger ones are refused
const MAX_FILE: u64 = 1024 * 1024;

/// Longest text shown inline; longer files are attached instead
const INLINE: usize = 1800;

/// Read a file from the server's files directory
#[command]
pub async fn file(
    ctx: Ctx<Host<Server>>,
    #[option("File name", autocomplete = file_names, max_len = 100)] name: &str,
) -> Result<Ephemeral<InteractionResponse>, HandlerError> {
    let path = resolve(&ctx.state().files_dir, name).await?;
    let size = tokio::fs::metadata(&path).await?.len();
    if size > MAX_FILE {
        return Err(HandlerError::user(format!(
            "`{name}` is {} KB; the limit is {} KB.",
            size / 1024,
            MAX_FILE / 1024
        )));
    }
    let bytes = tokio::fs::read(&path).await?;
    let response = match std::str::from_utf8(&bytes) {
        Ok(text) if text.len() <= INLINE => {
            InteractionResponse::respond_with_message(format!("`{name}`\n```\n{text}\n```"))
        }
        // Too long or not text: attached (the Adapter sends it through the callback endpoint)
        _ => InteractionResponse::respond_with_message(format!("`{name}`, {size} bytes:"))
            .with_file(File::new(name, bytes)),
    };
    Ok(Ephemeral(response))
}

/// Suggests the files in the files directory while the user types
#[autocomplete]
pub async fn file_names(ctx: Ctx<Host<Server>>, input: &str) -> Result<Vec<String>, HandlerError> {
    let mut entries = tokio::fs::read_dir(&ctx.state().files_dir).await?;
    let mut names = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_file() {
            continue;
        }
        if let Some(name) = entry.file_name().to_str()
            && is_plain_name(name)
            && name.to_lowercase().starts_with(&input.to_lowercase())
        {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// The file `name` inside `dir`, refusing anything that would leave it (`..`, separators,
/// absolute paths, hidden files, symlinks pointing out)
pub async fn resolve(dir: &Path, name: &str) -> Result<PathBuf, HandlerError> {
    let not_found = || HandlerError::user(format!("There is no file named `{name}`."));
    if !is_plain_name(name) {
        return Err(not_found());
    }
    let dir = tokio::fs::canonicalize(dir).await?;
    let path = tokio::fs::canonicalize(dir.join(name))
        .await
        .map_err(|_| not_found())?;
    if !path.starts_with(&dir) || !tokio::fs::metadata(&path).await?.is_file() {
        return Err(not_found());
    }
    Ok(path)
}

/// One path component naming a visible file
fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.contains(['/', '\\', ':'])
        && Path::new(name).components().count() == 1
}

/// How long this server has been running
#[command]
pub fn uptime(ctx: Ctx<Host<Server>>) -> String {
    let up = ctx.state().started.elapsed();
    format!(
        "Up for {} (process {}, {} worker threads).",
        human(up),
        std::process::id(),
        tokio::runtime::Handle::current().metrics().num_workers()
    )
}

fn human(duration: Duration) -> String {
    let secs = duration.as_secs();
    match secs {
        0..60 => format!("{secs}s"),
        60..3600 => format!("{}m {}s", secs / 60, secs % 60),
        _ => format!("{}h {}m", secs / 3600, secs % 3600 / 60),
    }
}

/// Count the primes up to a number (a Deferred Response)
#[command]
pub fn primes(
    ctx: Ctx<Host<Server>>,
    #[option("Count primes up to this", min = 2, max = 50_000_000)] up_to: i64,
) -> Deferred {
    // Answers within Discord's 3 seconds with "thinking..." (type 5); the closure then runs as
    // Background Work (a tokio task the shutdown grace period waits for), and what it returns
    // replaces that reply. It owns its `DeferredCtx`, so it can be moved to any thread.
    ctx.defer(async move |ctx| {
        let up_to = usize::try_from(up_to).map_err(HandlerError::new)?;
        // Progress while it works: the original response can be edited any number of times
        let working = format!("Sieving up to {up_to} on a blocking thread...");
        ctx.client()
            .edit_original(&EditMessage {
                content: Some(&working),
                ..Default::default()
            })
            .await?;

        let started = Instant::now();
        // CPU-bound work goes to tokio's blocking pool, so other Interactions keep their threads
        let (count, largest) = tokio::task::spawn_blocking(move || sieve(up_to))
            .await
            .map_err(HandlerError::new)?;
        let took = started.elapsed().as_millis();

        // The final edit: a success embed in the App's theme, like a direct reply
        Ok::<_, HandlerError>(ctx.theme().success(format!(
            "{count} primes up to {up_to}, the largest {largest}.
Counted in {took} ms."
        )))
    })
}

/// (how many primes are ≤ `n`, the largest)
fn sieve(n: usize) -> (usize, usize) {
    let mut composite = vec![false; n + 1];
    let (mut count, mut largest) = (0, 0);
    for i in 2..=n {
        if composite[i] {
            continue;
        }
        count += 1;
        largest = i;
        for multiple in (i * i..=n).step_by(i) {
            composite[multiple] = true;
        }
    }
    (count, largest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sieve_counts() {
        assert_eq!(sieve(2), (1, 2));
        assert_eq!(sieve(100), (25, 97));
    }

    #[test]
    fn only_plain_names_resolve() {
        for name in ["hello.txt", "notes.md"] {
            assert!(is_plain_name(name), "{name}");
        }
        for name in [
            "",
            "..",
            "../Cargo.toml",
            "a/b",
            "a\\b",
            ".env",
            "C:x",
            "/etc/passwd",
        ] {
            assert!(!is_plain_name(name), "{name}");
        }
    }
}
