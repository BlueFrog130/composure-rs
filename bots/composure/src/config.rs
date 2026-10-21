//! The bot's settings (its Bot Data): wrangler.toml vars and secrets, read once per isolate.

use composure::types::Snowflake;

/// Where the bot posts and what it reads
#[derive(Debug, Clone)]
pub struct Config {
    /// `owner/name` of the GitHub repository whose releases are announced (`GITHUB_REPO`)
    pub repo: String,
    /// Read-only token for the GitHub API (`GITHUB_TOKEN` secret); unauthenticated requests share
    /// Workers' IPs and their 60 requests an hour
    pub github_token: Option<String>,
    /// The home server's announcement channel releases are posted to and followed from
    /// (`ANNOUNCE_CHANNEL_ID`)
    pub announce_channel: Snowflake,
    /// Private channel `/feedback` posts to (`FEEDBACK_CHANNEL_ID`)
    pub feedback_channel: Snowflake,
    /// Private channel installs are logged to (`LOG_CHANNEL_ID`)
    pub log_channel: Snowflake,
}

/// The repository when `GITHUB_REPO` is unset
pub const DEFAULT_REPO: &str = "BlueFrog130/composure-rs";

impl Config {
    /// From vars by name (`env.var` on Workers, a map in tests)
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Config, String> {
        let id = |name: &str| -> Result<Snowflake, String> {
            lookup(name)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("{name} is not set"))?
                .parse()
                .map_err(|_| format!("{name} is not a snowflake"))
        };
        Ok(Config {
            repo: lookup("GITHUB_REPO")
                .filter(|repo| !repo.is_empty())
                .unwrap_or_else(|| DEFAULT_REPO.into()),
            github_token: lookup("GITHUB_TOKEN").filter(|token| !token.is_empty()),
            announce_channel: id("ANNOUNCE_CHANNEL_ID")?,
            feedback_channel: id("FEEDBACK_CHANNEL_ID")?,
            log_channel: id("LOG_CHANNEL_ID")?,
        })
    }

    /// The repository's web page
    pub fn repo_url(&self) -> String {
        format!("https://github.com/{}", self.repo)
    }
}
