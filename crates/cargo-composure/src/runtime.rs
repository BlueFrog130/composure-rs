//! Building and serving the bot. A server is rebuilt by cargo and restarted from a copy of its
//! exe, so the running one keeps serving while cargo overwrites the original (Windows can't
//! overwrite a running exe). A Worker is rebuilt in development mode by the project's own command
//! and served by `wrangler dev`, which reloads when the build output changes.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
    time::{Duration, SystemTime},
};

use tokio::{
    process::{Child, Command},
    time::timeout,
};
use toml_edit::{DocumentMut, Item, TableLike};

use crate::config::Project;

/// The generated Wrangler config, next to the project's so its relative paths still resolve
pub const DEV_CONFIG: &str = ".composure-dev.wrangler.toml";
/// The copies of the server's exe, under the target directory
const RUN_DIR: &str = "composure-dev";
const STOP_LIMIT: Duration = Duration::from_secs(5);

/// How a bot is built and kept running; tests record the calls
pub trait Runtime: Send {
    /// Builds the bot, printing the compiler's output. Whether it succeeded.
    fn build(&mut self) -> impl Future<Output = bool> + Send;

    /// Serves the latest build. `restart`: even if the build didn't change, because its
    /// environment did. Whether anything new is being served.
    fn swap(&mut self, restart: bool) -> impl Future<Output = Result<bool, String>> + Send;

    /// Resolves when the bot stops on its own, with how it ended; never without a running bot
    fn exited(&mut self) -> impl Future<Output = String> + Send;

    /// Stops the bot
    fn stop(&mut self) -> impl Future<Output = ()> + Send;
}

fn cargo() -> String {
    env::var("CARGO").unwrap_or_else(|_| "cargo".to_string())
}

/// `command` in the platform's shell, as Wrangler runs a build command
fn shell(command: &str) -> StdCommand {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut shell = StdCommand::new("cmd");
        shell.arg("/C").raw_arg(command);
        shell
    }
    #[cfg(not(windows))]
    {
        let mut shell = StdCommand::new("sh");
        shell.args(["-c", command]);
        shell
    }
}

/// A process with children of its own, in a group [`stop_tree`] can reach
fn tree(mut command: StdCommand) -> Command {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.stdin(Stdio::null());
    let mut command = Command::from(command);
    command.kill_on_drop(true);
    command
}

/// Stops `child` and everything it started: `npx` leaves Node and workerd behind otherwise
async fn stop_tree(child: &mut Child) {
    if let Some(id) = child.id() {
        let id = id.to_string();
        #[cfg(windows)]
        let stopper = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &id])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;
        #[cfg(not(windows))]
        let stopper = Command::new("kill")
            .args(["-s", "TERM", "--", &format!("-{id}")])
            .stderr(Stdio::null())
            .status()
            .await;
        if stopper.is_ok() && timeout(STOP_LIMIT, child.wait()).await.is_ok() {
            return;
        }
    }
    let _ = child.kill().await;
}

fn describe(status: io::Result<std::process::ExitStatus>) -> String {
    status.map_or_else(|e| e.to_string(), |s| s.to_string())
}

/// A bot on a hosted server: the package's server bin
pub struct Server {
    package: String,
    bin: String,
    dir: PathBuf,
    port: u16,
    /// Where cargo leaves the exe
    built: PathBuf,
    run_dir: PathBuf,
    child: Option<Child>,
    /// The copy `child` runs
    running: Option<PathBuf>,
    /// The modification time of the build being served
    served: Option<SystemTime>,
    copies: u32,
}

impl Server {
    pub fn new(project: &Project, bin: &str) -> Self {
        let exe = format!("{bin}{}", env::consts::EXE_SUFFIX);
        let run_dir = project.target_dir.join(RUN_DIR);
        // Left by a run that didn't end cleanly
        let _ = fs::remove_dir_all(&run_dir);
        Self {
            package: project.package.clone(),
            bin: bin.to_string(),
            dir: project.dir.clone(),
            port: project.port,
            built: project.target_dir.join("debug").join(exe),
            run_dir,
            child: None,
            running: None,
            served: None,
            copies: 0,
        }
    }

    fn copy_exe(&mut self) -> io::Result<PathBuf> {
        fs::create_dir_all(&self.run_dir)?;
        self.copies += 1;
        let name = format!("{}-{}{}", self.bin, self.copies, env::consts::EXE_SUFFIX);
        let copy = self.run_dir.join(name);
        fs::copy(&self.built, &copy)?;
        Ok(copy)
    }

    async fn stop_running(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill().await;
        }
        if let Some(copy) = self.running.take() {
            let _ = fs::remove_file(copy);
        }
    }
}

impl Runtime for Server {
    async fn build(&mut self) -> bool {
        // Only the server: Sync's `cargo run` builds its own bin, never while it is running
        let status = Command::new(cargo())
            .args(["build", "-p", &self.package, "--bin", &self.bin])
            .current_dir(&self.dir)
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .status()
            .await;
        status.is_ok_and(|status| status.success())
    }

    async fn swap(&mut self, restart: bool) -> Result<bool, String> {
        let built = fs::metadata(&self.built)
            .and_then(|m| m.modified())
            .map_err(|e| format!("{}: {e}", self.built.display()))?;
        if self.child.is_some() && self.served == Some(built) && !restart {
            return Ok(false);
        }
        let copy = self
            .copy_exe()
            .map_err(|e| format!("copying {}: {e}", self.built.display()))?;
        // Stopped before the next one binds its port
        self.stop_running().await;
        let child = Command::new(&copy)
            .current_dir(&self.dir)
            .env("PORT", self.port.to_string())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| format!("starting {}: {e}", self.bin))?;
        self.child = Some(child);
        self.running = Some(copy);
        self.served = Some(built);
        Ok(true)
    }

    async fn exited(&mut self) -> String {
        let Some(child) = &mut self.child else {
            return std::future::pending().await;
        };
        let status = child.wait().await;
        self.child = None;
        format!("{} exited ({})", self.bin, describe(status))
    }

    async fn stop(&mut self) {
        self.stop_running().await;
        let _ = fs::remove_dir_all(&self.run_dir);
    }
}

/// The Worker's development build, and the project's config without it
#[derive(Debug, PartialEq, Eq)]
struct DevBuild {
    command: String,
    /// `[build] cwd`
    cwd: Option<String>,
    config: String,
}

/// Takes the project's build command, in development mode: it stays the one place that pins
/// `worker-build`. `[build]` leaves the config so `wrangler dev` only serves, and doesn't race the
/// Dev Loop's own build with a slow release one.
fn dev_build(wrangler_toml: &str, env: Option<&str>) -> Result<DevBuild, String> {
    let mut config: DocumentMut = wrangler_toml.parse().map_err(|e| format!("{e}"))?;
    let in_env = env.and_then(|name| config.get("env")?.get(name)?.get("build"));
    let build = in_env
        .or_else(|| config.get("build"))
        .ok_or("wrangler.toml has no [build] command")?;
    let field = |name| build.get(name).and_then(Item::as_str).map(str::to_string);
    let command = field("command")
        .ok_or("wrangler.toml has no [build] command")?
        .replace("--release", "--dev");
    let cwd = field("cwd");

    config.remove("build");
    if let Some(table) = env.and_then(|name| {
        config
            .get_mut("env")?
            .get_mut(name)?
            .as_table_like_mut()
            .map(|t: &mut dyn TableLike| t)
    }) {
        table.remove("build");
    }
    Ok(DevBuild {
        command,
        cwd,
        config: format!("# Generated by `cargo composure dev`\n{config}"),
    })
}

/// A bot on Cloudflare Workers
pub struct Workers {
    dir: PathBuf,
    command: String,
    build_dir: PathBuf,
    config: PathBuf,
    port: u16,
    wrangler_env: Option<String>,
    child: Option<Child>,
}

impl Workers {
    pub fn new(project: &Project, wrangler_toml: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(wrangler_toml)
            .map_err(|e| format!("{}: {e}", wrangler_toml.display()))?;
        let build = dev_build(&text, project.wrangler_env.as_deref())?;
        let config = project.dir.join(DEV_CONFIG);
        fs::write(&config, build.config).map_err(|e| format!("{}: {e}", config.display()))?;
        Ok(Self {
            dir: project.dir.clone(),
            command: build.command,
            build_dir: project.dir.join(build.cwd.unwrap_or_default()),
            config,
            port: project.port,
            wrangler_env: project.wrangler_env.clone(),
            child: None,
        })
    }

    fn wrangler_dev(&self) -> String {
        let mut command = format!(
            "npx --yes wrangler dev -c {DEV_CONFIG} --port {} --show-interactive-dev-session=false",
            self.port
        );
        if let Some(env) = &self.wrangler_env {
            command.push_str(&format!(" --env {env}"));
        }
        command
    }

    async fn stop_running(&mut self) {
        if let Some(mut child) = self.child.take() {
            stop_tree(&mut child).await;
        }
    }
}

impl Runtime for Workers {
    async fn build(&mut self) -> bool {
        let mut build = shell(&self.command);
        build.current_dir(&self.build_dir).stdin(Stdio::null());
        let status = Command::from(build).kill_on_drop(true).status().await;
        status.is_ok_and(|status| status.success())
    }

    async fn swap(&mut self, restart: bool) -> Result<bool, String> {
        if restart {
            self.stop_running().await;
        }
        if self.child.is_none() {
            let mut wrangler = shell(&self.wrangler_dev());
            wrangler.current_dir(&self.dir);
            let child = tree(wrangler)
                .spawn()
                .map_err(|e| format!("starting wrangler: {e}"))?;
            self.child = Some(child);
        }
        Ok(true)
    }

    async fn exited(&mut self) -> String {
        let Some(child) = &mut self.child else {
            return std::future::pending().await;
        };
        let status = child.wait().await;
        self.child = None;
        format!("wrangler exited ({})", describe(status))
    }

    async fn stop(&mut self) {
        self.stop_running().await;
        let _ = fs::remove_file(&self.config);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WRANGLER: &str = r#"name = "bot"
main = "build/worker/shim.mjs"

# worker-build compiles the crate
[build]
command = "cargo install -q worker-build@0.8.7 && worker-build --release"

[[kv_namespaces]]
binding = "NOTES"
"#;

    #[test]
    fn takes_the_projects_command_in_development_mode() {
        let build = dev_build(WRANGLER, None).unwrap();
        assert_eq!(
            build.command,
            "cargo install -q worker-build@0.8.7 && worker-build --dev"
        );
        assert_eq!(build.cwd, None);
    }

    #[test]
    fn the_config_keeps_everything_but_the_build() {
        let config = dev_build(WRANGLER, None).unwrap().config;
        assert!(!config.contains("[build]"), "{config}");
        assert!(!config.contains("worker-build --release"), "{config}");
        assert!(
            config.contains(r#"main = "build/worker/shim.mjs""#),
            "{config}"
        );
        assert!(config.contains("[[kv_namespaces]]"), "{config}");
    }

    #[test]
    fn a_wrangler_env_can_have_its_own_build() {
        let toml = r#"
[build]
command = "worker-build --release"

[env.dev]
name = "bot-dev"

[env.dev.build]
command = "custom --release"
cwd = "worker"
"#;
        let build = dev_build(toml, Some("dev")).unwrap();
        assert_eq!(build.command, "custom --dev");
        assert_eq!(build.cwd.as_deref(), Some("worker"));
        assert!(!build.config.contains("custom"), "{}", build.config);
        assert!(
            build.config.contains(r#"name = "bot-dev""#),
            "{}",
            build.config
        );

        assert_eq!(
            dev_build(toml, Some("production")).unwrap().command,
            "worker-build --dev"
        );
    }

    #[test]
    fn a_config_without_a_build_is_an_error() {
        assert!(dev_build("name = \"bot\"", None).is_err());
    }
}
