//! dev Sync: the bot's own Sync bin, sent to the dev guild. Definitions only exist compiled into
//! the bot, so its command line is the whole interface.

use std::{env, path::PathBuf, process::Stdio, sync::Arc};

use composure::{credentials::APPLICATION_ID_VAR, types::Snowflake};
use tokio::{process::Command, sync::Notify};

use crate::stage::{info, warning};

/// What the Sync bin prints when it changed something
const SYNCED: &str = "Synced: ";

pub struct DevSync {
    pub package: String,
    pub bin: String,
    pub dir: PathBuf,
    pub application: Snowflake,
    pub guild: Snowflake,
}

impl DevSync {
    /// Syncs the latest source; what changed, `None` when nothing did
    async fn run(&self) -> Result<Option<String>, String> {
        let output = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".to_string()))
            .args(["run", "-q", "-p", &self.package, "--bin", &self.bin, "--"])
            .args([
                "--guild",
                &self.guild.to_string(),
                "--yes",
                "--color",
                "never",
            ])
            .current_dir(&self.dir)
            // Over `.env`: dev Sync only ever writes to the Dev Application
            .env(APPLICATION_ID_VAR, self.application.to_string())
            .stdin(Stdio::null())
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|e| format!("starting cargo: {e}"))?;
        let text = String::from_utf8_lossy(&output.stderr).into_owned()
            + &String::from_utf8_lossy(&output.stdout);
        match output.status.success() {
            true => Ok(changes(&text)),
            false => Err(text.trim().to_string()),
        }
    }

    /// Runs after each request; requests made during a run collapse into one follow-up
    pub async fn serve(self, requests: Arc<Notify>) {
        let mut reported = false;
        loop {
            requests.notified().await;
            match self.run().await {
                Ok(Some(changes)) => {
                    reported = true;
                    info!("dev Sync to guild {}: {changes}", self.guild);
                }
                Ok(None) if !reported => {
                    reported = true;
                    info!("dev Sync to guild {}: up to date", self.guild);
                }
                Ok(None) => {}
                Err(output) => warning!("dev Sync failed:\n{output}"),
            }
        }
    }
}

fn changes(output: &str) -> Option<String> {
    let (_, changes) = output.rsplit_once(SYNCED)?;
    Some(changes.lines().next()?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_the_sync_bin_changed() {
        let output = "┌  composure sync \n│\n◇  Guild 1\n│\n└  Synced: 2 created · 1 deleted\n\n";
        assert_eq!(changes(output).as_deref(), Some("2 created · 1 deleted"));
        assert_eq!(changes("└  Discord is up to date\n"), None);
    }
}
