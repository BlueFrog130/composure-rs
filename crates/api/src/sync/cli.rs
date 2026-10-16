//! The Sync command line: clap for arguments, cliclack for output and the confirmation prompt.
//!
//! In a terminal it shows the Sync Plan and asks before applying. Anywhere else (CI, pipes) it
//! never prompts: it applies only with `--yes` and fails otherwise, so a forgotten flag can't
//! hang a pipeline or silently skip a deploy.

use std::{
    fmt::Display,
    io::{self, IsTerminal},
    process::ExitCode,
};

use clap::{Parser, ValueEnum};
use cliclack::{confirm, intro, log, outro, outro_cancel, spinner};
use composure::command::CommandSetDefinition;
use console::{StyledObject, style};

use super::{
    Change, ChangeCounts, SyncConfig, SyncError, SyncPlan, Target, apply_target, guild_id, plan,
};
use crate::DiscordClient;

/// `--check` found changes
const OUT_OF_DATE: u8 = 2;

/// Make Discord's commands match the bot's Command Set
#[derive(Debug, Parser)]
#[command(
    name = "sync",
    after_help = "Reads DISCORD_TOKEN, DISCORD_APPLICATION_ID, and COMPOSURE_SCOPE_<NAME>=<guild id>[,<guild id>] from the environment.\n\nExit status: 0 synced or up to date, 1 failed or cancelled, 2 out of date (--check)."
)]
struct Args {
    /// Show the Sync Plan without applying it
    #[arg(long)]
    dry_run: bool,

    /// Show the Sync Plan and exit with status 2 if Discord is out of date (for CI)
    #[arg(long, conflicts_with_all = ["dry_run", "yes"])]
    check: bool,

    /// Apply without asking; required to apply outside a terminal
    #[arg(short, long, conflicts_with = "dry_run")]
    yes: bool,

    /// Send every command to this guild instead, ignoring Scopes and leaving global commands
    /// alone (for testing)
    #[arg(long, value_name = "GUILD_ID", value_parser = parse_guild)]
    guild: Option<String>,

    /// When to use colors
    #[arg(long, value_enum, value_name = "WHEN", default_value_t = Color::Auto)]
    color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Color {
    Auto,
    Always,
    Never,
}

fn parse_guild(id: &str) -> Result<String, String> {
    guild_id(id).map_err(|()| format!("`{id}` is not a guild id"))
}

/// Entry point for a Sync bin: parses the command line, reads [`SyncConfig::from_env`], shows
/// the Sync Plan, and applies it after confirmation.
///
/// Flags: `--dry-run` only shows the plan, `--check` also exits with status 2 when there are
/// changes, `--yes` applies without asking (required outside a terminal), `--guild <id>` sends
/// every Command to one guild (see [`SyncConfig::override_guild`]), `--color` forces colors on
/// or off. `--help` lists them.
///
/// ```ignore
/// // src/bin/sync.rs
/// fn main() -> std::process::ExitCode {
///     composure_api::sync_from_env(&my_bot::Bot)
/// }
/// ```
pub fn sync_from_env<C: CommandSetDefinition>(_commands: &C) -> ExitCode {
    // Not `Args::parse()`: clap exits 2 on usage errors, which `--check` means out of date
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            let _ = error.print();
            return match error.use_stderr() {
                true => ExitCode::FAILURE,
                false => ExitCode::SUCCESS, // --help
            };
        }
    };
    if args.color != Color::Auto {
        let enabled = args.color == Color::Always;
        console::set_colors_enabled(enabled);
        console::set_colors_enabled_stderr(enabled);
    }

    let ui = Ui::detect();
    match run::<C>(&args, &ui) {
        Ok(code) => code,
        Err(Abort::Failed(error)) => {
            let _ = log::error(error);
            let _ = outro_cancel("Sync failed");
            ExitCode::FAILURE
        }
        Err(Abort::Cancelled) => {
            let _ = outro_cancel("Cancelled, nothing was changed");
            ExitCode::FAILURE
        }
        Err(Abort::Io(error)) => {
            eprintln!("sync failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run<C: CommandSetDefinition>(args: &Args, ui: &Ui) -> Result<ExitCode, Abort> {
    intro(style(" composure sync ").on_cyan().black())?;

    let mut config = SyncConfig::from_env()?;
    config.override_guild = args.guild.clone();
    if let Some(guild) = &config.override_guild {
        log::warning(format!(
            "Sending every command to guild {guild}; Scopes and global commands are left alone"
        ))?;
    }

    let plan = ui.task(
        "Fetching commands from Discord",
        || plan(C::definitions(), &config),
        |plan| format!("Compared {}", count(plan.targets.len(), "target")),
    )?;
    show(&plan)?;

    if !plan.has_changes() {
        outro("Discord is up to date")?;
        return Ok(ExitCode::SUCCESS);
    }
    let counts = plan.counts();
    if args.check {
        outro_cancel(format!(
            "Discord is out of date: {}",
            summary(counts, Tense::Planned)
        ))?;
        return Ok(ExitCode::from(OUT_OF_DATE));
    }
    if args.dry_run {
        outro(style("Dry run, nothing was changed").dim())?;
        return Ok(ExitCode::SUCCESS);
    }
    if counts.delete > 0 {
        log::warning(format!(
            "{} will be deleted from Discord",
            count(counts.delete, "command")
        ))?;
    }

    if !args.yes {
        if !ui.interactive {
            log::error(
                "Not running in a terminal, so there is no one to confirm.\n\
                 Pass --yes to apply, or --dry-run / --check to only show the plan.",
            )?;
            outro_cancel("Nothing was changed")?;
            return Ok(ExitCode::FAILURE);
        }
        // Deleting is the one change that loses something; make it opt-in
        let proceed = confirm(format!("Apply {}?", count(counts.total(), "change")))
            .initial_value(counts.delete == 0)
            .interact()?;
        if !proceed {
            return Err(Abort::Cancelled);
        }
    }

    let client = DiscordClient::new(&config.token, &config.application_id)?;
    for target in plan.targets.iter().filter(|t| !t.changes.is_empty()) {
        ui.task(
            format!("Updating {}", name(&target.target)),
            || apply_target(&client, target),
            |()| {
                format!(
                    "Updated {} {}",
                    name(&target.target),
                    style(format!("({})", count(target.commands.len(), "command"))).dim()
                )
            },
        )?;
    }

    outro(format!("Synced: {}", summary(counts, Tense::Done)))?;
    Ok(ExitCode::SUCCESS)
}

/// One block per target: changed ones list their changes, up-to-date ones are a dim line
fn show(plan: &SyncPlan) -> io::Result<()> {
    for target in &plan.targets {
        let name = style(name(&target.target)).bold();
        if target.changes.is_empty() {
            let detail = format!("up to date, {}", count(target.unchanged, "command"));
            log::remark(format!("{name}  {}", style(detail).dim()))?;
            continue;
        }

        let mut block = name.to_string();
        if target.unchanged > 0 {
            let detail = format!("{} unchanged", target.unchanged);
            block.push_str(&format!("  {}", style(detail).dim()));
        }
        for change in &target.changes {
            block.push_str(&format!("\n  {}", paint(change, change.to_string())));
        }
        log::step(block)?;
    }

    if plan.has_changes() {
        log::info(summary(plan.counts(), Tense::Planned))?;
    }
    Ok(())
}

fn paint<D>(change: &Change, text: D) -> StyledObject<D> {
    match change {
        Change::Create(_) => style(text).green(),
        Change::Update(_) => style(text).yellow(),
        Change::Delete(_) => style(text).red(),
    }
}

#[derive(Clone, Copy)]
enum Tense {
    Planned,
    Done,
}

/// `2 to create · 1 to delete`, colored like the changes; zero counts are left out
fn summary(counts: ChangeCounts, tense: Tense) -> String {
    let parts = [
        (
            counts.create,
            Change::Create(String::new()),
            "create",
            "created",
        ),
        (
            counts.update,
            Change::Update(String::new()),
            "update",
            "updated",
        ),
        (
            counts.delete,
            Change::Delete(String::new()),
            "delete",
            "deleted",
        ),
    ];
    parts
        .into_iter()
        .filter(|(n, ..)| *n > 0)
        .map(|(n, change, planned, done)| {
            let text = match tense {
                Tense::Planned => format!("{n} to {planned}"),
                Tense::Done => format!("{n} {done}"),
            };
            paint(&change, text).to_string()
        })
        .collect::<Vec<_>>()
        .join(&style(" · ").dim().to_string())
}

fn name(target: &Target) -> String {
    match target {
        Target::Global => "Global".into(),
        Target::Guild(id) => format!("Guild {id}"),
    }
}

fn count(n: usize, noun: &str) -> String {
    match n {
        1 => format!("1 {noun}"),
        n => format!("{n} {noun}s"),
    }
}

/// How the terminal can be used
struct Ui {
    /// stderr is a terminal: spinners can redraw in place
    animated: bool,
    /// Someone can answer a prompt: a terminal on both ends and not CI
    interactive: bool,
}

impl Ui {
    fn detect() -> Self {
        let animated = io::stderr().is_terminal();
        let ci = std::env::var("CI").is_ok_and(|v| !matches!(v.as_str(), "" | "0" | "false"));
        Ui {
            animated,
            interactive: animated && io::stdin().is_terminal() && !ci,
        }
    }

    /// Runs `work` behind a spinner, or logs only its outcome when the output isn't a terminal
    /// (where a spinner would print nothing).
    fn task<T>(
        &self,
        start: impl Display,
        work: impl FnOnce() -> Result<T, SyncError>,
        done: impl FnOnce(&T) -> String,
    ) -> Result<T, Abort> {
        if !self.animated {
            let value = work()?;
            log::success(done(&value))?;
            return Ok(value);
        }

        let spinner = spinner();
        spinner.start(&start);
        match work() {
            Ok(value) => {
                spinner.stop(done(&value));
                Ok(value)
            }
            Err(error) => {
                spinner.error(&start);
                Err(error.into())
            }
        }
    }
}

enum Abort {
    Failed(SyncError),
    /// Declined the prompt, or Esc / Ctrl-C during it
    Cancelled,
    /// Writing to the terminal failed
    Io(io::Error),
}

impl From<SyncError> for Abort {
    fn from(error: SyncError) -> Self {
        Abort::Failed(error)
    }
}

impl From<crate::Error> for Abort {
    fn from(error: crate::Error) -> Self {
        Abort::Failed(error.into())
    }
}

impl From<io::Error> for Abort {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::Interrupted => Abort::Cancelled,
            _ => Abort::Io(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn args_are_consistent() {
        Args::command().debug_assert();
    }

    #[test]
    fn flags_parse() {
        let args = Args::try_parse_from(["sync", "-y", "--guild", "798662131062931547"]).unwrap();
        assert!(args.yes);
        assert_eq!(args.guild.as_deref(), Some("798662131062931547"));

        assert!(Args::try_parse_from(["sync", "--guild", "abc"]).is_err());
        assert!(Args::try_parse_from(["sync", "--check", "--yes"]).is_err());
        assert!(Args::try_parse_from(["sync", "--dry-run", "--yes"]).is_err());
    }

    #[test]
    fn summary_skips_zero_counts() {
        console::set_colors_enabled(false);
        let counts = ChangeCounts {
            create: 2,
            update: 0,
            delete: 1,
        };
        assert_eq!(summary(counts, Tense::Planned), "2 to create · 1 to delete");
        assert_eq!(summary(counts, Tense::Done), "2 created · 1 deleted");
    }
}
