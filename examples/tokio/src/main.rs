//! The example bot served from a hosted server on tokio: `cargo run -p composure_example_tokio`.
//! Reads `.env` next to this crate's Cargo.toml (`DISCORD_PUBLIC_KEY`, optional `DISCORD_TOKEN`,
//! `DISCORD_APPLICATION_ID`, `GREETING`, `FILES_DIR`, `PORT`); variables already set in the
//! environment win. Logs go to stderr, filtered by `RUST_LOG` (default `info,composure=debug`).

use composure_adapter_tokio::TokioInteractionBot;
use composure_example_tokio::{DEFAULT_FILES_DIR, Server, TokioBot};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), composure_adapter_tokio::Error> {
    let _ = dotenvy::from_path(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,composure=debug")),
        )
        .init();

    let files_dir = std::env::var("FILES_DIR").unwrap_or_else(|_| DEFAULT_FILES_DIR.into());
    TokioInteractionBot::from_env()?
        .with_handler(TokioBot)
        .with_data(Server::new(files_dir))
        .serve()
        .await
}
