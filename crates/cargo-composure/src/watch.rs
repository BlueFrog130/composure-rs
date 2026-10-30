//! Saves that matter: anything under a local package's `src/`, its `Cargo.toml` or `build.rs`, and
//! the bot's `.env`. Debounced, so an editor's burst of writes is one rebuild.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::{sync::mpsc, time::timeout};

/// Debounces saves into one rebuild
const DEBOUNCE: Duration = Duration::from_millis(300);
const ENV_FILE: &str = ".env";

/// One or more saves
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Change {
    /// `.env` changed: the bot restarts even if its build didn't
    pub env: bool,
}

impl Change {
    pub fn merge(self, other: Self) -> Self {
        Self {
            env: self.env || other.env,
        }
    }
}

/// Whether a write to `path` matters: `None`, or the change it is. `sources` are the local
/// packages, `bot` the bot's own.
fn classify(sources: &[PathBuf], bot: &Path, path: &Path) -> Option<Change> {
    if path.parent() == Some(bot) && path.file_name().is_some_and(|name| name == ENV_FILE) {
        return Some(Change { env: true });
    }
    let matters = sources.iter().any(|source| {
        path.strip_prefix(source).is_ok_and(|relative| {
            relative.starts_with("src")
                || relative == Path::new("Cargo.toml")
                || relative == Path::new("build.rs")
        })
    });
    matters.then_some(Change::default())
}

/// Watches `sources`; dropping the watcher ends the channel
pub fn watch(
    sources: Vec<PathBuf>,
    bot: PathBuf,
) -> notify::Result<(RecommendedWatcher, mpsc::Receiver<Change>)> {
    let (saves, mut received) = mpsc::unbounded_channel();
    let roots = sources.clone();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let Ok(event) = event else { return };
        if matches!(event.kind, EventKind::Access(_)) {
            return;
        }
        for path in &event.paths {
            if let Some(change) = classify(&sources, &bot, path) {
                let _ = saves.send(change);
            }
        }
    })?;
    for root in &roots {
        watcher.watch(root, RecursiveMode::Recursive)?;
    }

    let (changes, debounced) = mpsc::channel(1);
    tokio::spawn(async move {
        while let Some(mut change) = received.recv().await {
            while let Ok(Some(next)) = timeout(DEBOUNCE, received.recv()).await {
                change = change.merge(next);
            }
            if changes.send(change).await.is_err() {
                return;
            }
        }
    });
    Ok((watcher, debounced))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classified(path: &str) -> Option<Change> {
        let sources = [PathBuf::from("/repo/bot"), PathBuf::from("/repo/shared")];
        classify(&sources, Path::new("/repo/bot"), Path::new(path))
    }

    #[test]
    fn sources_and_manifests_matter() {
        for path in [
            "/repo/bot/src/lib.rs",
            "/repo/bot/src/assets/banner.txt",
            "/repo/bot/Cargo.toml",
            "/repo/bot/build.rs",
            "/repo/shared/src/lib.rs",
        ] {
            assert_eq!(classified(path), Some(Change::default()), "{path}");
        }
    }

    #[test]
    fn the_bots_env_restarts_it() {
        assert_eq!(classified("/repo/bot/.env"), Some(Change { env: true }));
        assert_eq!(classified("/repo/shared/.env"), None);
    }

    #[test]
    fn build_output_and_everything_else_is_ignored() {
        for path in [
            "/repo/bot/target/debug/bot.exe",
            "/repo/bot/build/worker/shim.mjs",
            "/repo/bot/.wrangler/state/v3/kv/db.sqlite",
            "/repo/bot/tests/offline.rs",
            "/repo/bot/README.md",
            "/repo/other/src/lib.rs",
        ] {
            assert_eq!(classified(path), None, "{path}");
        }
    }
}
