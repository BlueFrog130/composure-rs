//! Registers the bot's Commands with Discord: `cargo run --bin sync`. Reads `DISCORD_TOKEN` and
//! `DISCORD_APPLICATION_ID` from the environment or `.env`; `--help` lists the flags.

fn main() -> std::process::ExitCode {
    // Variables already set in the environment win over the file; no file is fine
    let _ = dotenvy::dotenv();
    composure_sync::sync_from_env(&{{crate_name}}::Bot)
}
