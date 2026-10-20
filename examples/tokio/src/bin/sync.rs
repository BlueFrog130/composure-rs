//! Syncs the tokio bot's commands (the shared ones plus `/file`, `/uptime`, `/primes`) with
//! Discord: `cargo run -p composure_example_tokio --bin sync-tokio`. Reads `.env` next to this
//! crate's Cargo.toml; see the crate README for the vars and flags.

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

    composure_sync::sync_from_env(&composure_example_tokio::TokioBot)
}
