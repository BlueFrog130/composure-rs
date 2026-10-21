//! Serves the bot: `cargo run`. Reads `DISCORD_PUBLIC_KEY` (and optionally `DISCORD_TOKEN`,
//! `DISCORD_APPLICATION_ID`, `PORT`) from the environment or `.env`. Logs go to stderr, filtered
//! by `RUST_LOG` (default `info`).

use composure_adapter_tokio::TokioInteractionBot;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), composure_adapter_tokio::Error> {
    // Variables already set in the environment win over the file; no file is fine
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // 0.0.0.0:$PORT (8080) until Ctrl-C or SIGTERM, then up to 30s for in-flight work
    TokioInteractionBot::from_env()?
        .with_handler({{crate_name}}::Bot)
        .serve()
        .await
}
