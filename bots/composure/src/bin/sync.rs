//! Syncs the bot's commands with Discord (globally):
//! `cargo run -p composure_bot --bin sync-composure-bot`.
//! Reads `.env` next to this crate's Cargo.toml; see the crate README for the vars and flags.

#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    // Variables already set in the environment win over the file; no file is fine
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/.env");
    match dotenvy::from_path(path) {
        Ok(()) => {}
        Err(error) if error.not_found() => {}
        Err(error) => {
            eprintln!("sync failed: {path}: {error}");
            return std::process::ExitCode::FAILURE;
        }
    }

    composure_sync::sync_from_env(&composure_bot::ComposureBot)
}

#[cfg(target_arch = "wasm32")]
fn main() {}
