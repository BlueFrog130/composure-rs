#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod config;
mod dev;
mod dns;
mod repoint;
mod runtime;
mod stage;
mod sync;
mod tunnel;
mod watch;

use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use config::Project;

/// `cargo` runs this as `cargo-composure composure <args>`
#[derive(Debug, Parser)]
#[command(name = "cargo", bin_name = "cargo")]
enum Cargo {
    /// Tools for Composure Discord bots
    Composure(Composure),
}

#[derive(Debug, Args)]
#[command(version)]
struct Composure {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the bot through a tunnel, point the Dev Application at it, and rebuild it and sync
    /// its Commands to the dev guild on every save
    Dev(Dev),
}

#[derive(Debug, Args)]
#[command(
    after_help = "Settings live in [package.metadata.composure.dev]: application, guild, tunnel, port, bin, sync_bin, wrangler_env. Secrets (DISCORD_TOKEN, DISCORD_PUBLIC_KEY) come from .env, under the process environment."
)]
struct Dev {
    /// The bot's package, in a workspace (default: the current directory's)
    #[arg(short, long)]
    package: Option<String>,

    /// The Dev Application's id, instead of `application`
    #[arg(long, value_name = "ID")]
    application: Option<String>,

    /// The dev guild's id, instead of `guild`
    #[arg(long, value_name = "ID")]
    guild: Option<String>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let Cargo::Composure(composure) = Cargo::parse();
    match composure.command {
        Command::Dev(dev) => {
            match Project::load(dev.package.as_deref(), dev.application, dev.guild) {
                Ok(project) => dev::run(project).await,
                Err(error) => {
                    eprintln!("[composure] {error}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn args_are_consistent() {
        Cargo::command().debug_assert();
    }

    #[test]
    fn parses_as_cargo_runs_it() {
        let Cargo::Composure(composure) = Cargo::try_parse_from([
            "cargo-composure",
            "composure",
            "dev",
            "-p",
            "bot",
            "--guild",
            "1",
        ])
        .unwrap();
        let Command::Dev(dev) = composure.command;
        assert_eq!(dev.package.as_deref(), Some("bot"));
        assert_eq!(dev.guild.as_deref(), Some("1"));
    }
}
