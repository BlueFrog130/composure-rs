//! Sync: make Discord's stored Command Definitions match an App's Commands, at deploy time.
//!
//! Discord only shows the Commands it has stored for an application. This crate reads the
//! definitions of a [Composure](https://docs.rs/composure) App (`#[app]` implements
//! [`CommandSetDefinition`](composure::command::CommandSetDefinition)) and makes Discord's match
//! them, from a binary that runs on the host, never in the deployed bot:
//!
//! ```ignore
//! // src/bin/sync.rs
//! fn main() -> std::process::ExitCode {
//!     composure_sync::sync_from_env(&my_bot::Bot)
//! }
//! ```
//!
//! ```sh
//! cargo run --bin sync             # show the plan, apply after confirming
//! cargo run --bin sync -- --check  # CI: exit 2 when Discord is out of date
//! cargo run --bin sync -- --yes    # CI deploys: apply without asking
//! ```
//!
//! # Configuration
//!
//! | Env var | |
//! |---|---|
//! | `DISCORD_TOKEN` | The bot token |
//! | `DISCORD_APPLICATION_ID` | The application id |
//! | `COMPOSURE_SCOPE_<NAME>` | Comma-separated guild ids for Commands with `scope = "name"` (uppercased, `-` as `_`) |
//!
//! # How it works
//!
//! Each target (global, or one guild) gets every Command whose Scope maps to it: Commands without
//! a Scope are global, Scoped ones go to their Scope's guilds. A [`SyncPlan`] is computed by
//! comparing Definitions by name and type; targets with changes are overwritten with one bulk
//! request each, targets without changes are left alone. Every guild listed for a Scope is
//! synced, so a guild whose Commands were all removed is cleared; guilds not listed are never
//! touched.
//!
//! The command line (`sync_from_env`, clap and cliclack) is the default `cli` feature. Without
//! it, [`SyncConfig`], [`plan`] and [`apply`] remain, for other configuration sources or a custom
//! front end:
//!
//! ```no_run
//! # fn main() -> Result<(), composure_sync::SyncError> {
//! # struct Bot;
//! # impl composure::command::CommandSetDefinition for Bot {
//! #     fn definitions() -> Vec<composure::command::ScopedDefinition> { Vec::new() }
//! # }
//! use composure::command::CommandSetDefinition;
//!
//! let config = composure_sync::SyncConfig::from_env()?;
//! let plan = composure_sync::plan(Bot::definitions(), &config)?;
//! println!("{plan}");
//! if plan.has_changes() {
//!     composure_sync::apply(&plan, &config)?;
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    str::FromStr,
    time::Duration,
};

use composure::{
    command::ScopedDefinition,
    credentials::{APPLICATION_ID_VAR, BOT_TOKEN_VAR},
    definition::{ApplicationCommand, CommandDetails},
    rest::{self, BotClient, CommandScope, ReqwestTransport},
    types::Snowflake,
};
use pollster::block_on;
use serde_json::Value;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
pub use cli::sync_from_env;

const SCOPE_PREFIX: &str = "COMPOSURE_SCOPE_";

/// Where a set of Commands is stored on Discord
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// Global Commands
    Global,
    /// One guild's Commands, by guild id
    Guild(String),
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::Global => f.write_str("global"),
            Target::Guild(id) => write!(f, "guild {id}"),
        }
    }
}

/// Credentials and Scope mapping for a Sync
#[derive(Debug, Clone, Default)]
pub struct SyncConfig {
    /// The bot token
    pub token: String,
    /// The application id
    pub application_id: String,

    /// Scope name to guild ids. Every listed guild is synced, even if no Command maps to it
    /// anymore (which clears its Commands).
    pub scopes: HashMap<String, Vec<String>>,

    /// Send every Command to this guild instead, ignoring Scopes and leaving global
    /// Commands alone. For testing.
    pub override_guild: Option<String>,
}

impl SyncConfig {
    /// `DISCORD_TOKEN`, `DISCORD_APPLICATION_ID`, and one `COMPOSURE_SCOPE_<NAME>=id[,id]` per
    /// Scope. `<NAME>` is the Scope uppercased with `-` as `_`.
    pub fn from_env() -> Result<Self, SyncError> {
        Self::from_vars(std::env::vars())
    }

    /// [`SyncConfig::from_env`] from any `(name, value)` pairs, e.g. a `.env` file read by hand
    pub fn from_vars(vars: impl IntoIterator<Item = (String, String)>) -> Result<Self, SyncError> {
        let mut config = SyncConfig::default();
        for (key, value) in vars {
            if key == BOT_TOKEN_VAR {
                config.token = value;
            } else if key == APPLICATION_ID_VAR {
                config.application_id = value;
            } else if let Some(scope) = key.strip_prefix(SCOPE_PREFIX) {
                let guilds = value
                    .split(',')
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                    .map(|id| {
                        guild_id(id).map_err(|_| {
                            SyncError::Config(format!("{key}: `{id}` is not a guild id"))
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                config.scopes.insert(scope_key(scope), guilds);
            }
        }
        if config.token.is_empty() {
            return Err(SyncError::Config(format!("{BOT_TOKEN_VAR} is not set")));
        }
        if config.application_id.is_empty() {
            return Err(SyncError::Config(format!(
                "{APPLICATION_ID_VAR} is not set"
            )));
        }
        Ok(config)
    }
}

fn guild_id(id: &str) -> Result<String, ()> {
    Snowflake::from_str(id)
        .map(|s| s.to_string())
        .map_err(|_| ())
}

/// `dev-eu` and `DEV_EU` name the same Scope
fn scope_key(scope: &str) -> String {
    scope.to_ascii_uppercase().replace('-', "_")
}

/// Why a Sync could not be planned or applied
#[derive(Debug)]
pub enum SyncError {
    /// Missing or malformed configuration
    Config(String),
    /// A Command's Scope has no guilds configured
    UnmappedScope(String),
    /// Two Commands with the same name and type go to the same target
    Duplicate {
        /// Where both would go
        target: Target,
        /// Their name
        name: String,
    },
    /// A request to Discord failed
    Api(rest::Error),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::Config(message) => f.write_str(message),
            SyncError::UnmappedScope(scope) => write!(
                f,
                "scope `{scope}` has no guilds: set {SCOPE_PREFIX}{}=<guild id>",
                scope_key(scope)
            ),
            SyncError::Duplicate { target, name } => {
                write!(
                    f,
                    "{target}: more than one command named `{name}` of the same type"
                )
            }
            SyncError::Api(error) => write!(f, "{error}"),
        }
    }
}

impl From<rest::Error> for SyncError {
    fn from(error: rest::Error) -> Self {
        SyncError::Api(error)
    }
}

/// Splits Definitions into the Commands each target should end up with.
///
/// `contexts`, `integration_types`, and `dm_permission` only apply to global Commands, so they
/// are dropped for guild targets.
pub fn targets(
    definitions: Vec<ScopedDefinition>,
    config: &SyncConfig,
) -> Result<BTreeMap<Target, Vec<ApplicationCommand>>, SyncError> {
    let mut targets: BTreeMap<Target, Vec<ApplicationCommand>> = BTreeMap::new();

    if let Some(guild) = &config.override_guild {
        let commands = definitions
            .into_iter()
            .map(|d| for_guild(d.command))
            .collect();
        targets.insert(Target::Guild(guild.clone()), commands);
    } else {
        targets.insert(Target::Global, Vec::new());
        for guilds in config.scopes.values() {
            for guild in guilds {
                targets.entry(Target::Guild(guild.clone())).or_default();
            }
        }
        for definition in definitions {
            match definition.scope {
                None => targets
                    .entry(Target::Global)
                    .or_default()
                    .push(definition.command),
                Some(scope) => {
                    let guilds = config
                        .scopes
                        .get(&scope_key(scope))
                        .filter(|guilds| !guilds.is_empty())
                        .ok_or_else(|| SyncError::UnmappedScope(scope.to_string()))?;
                    for guild in guilds {
                        targets
                            .entry(Target::Guild(guild.clone()))
                            .or_default()
                            .push(for_guild(definition.command.clone()));
                    }
                }
            }
        }
    }

    for (target, commands) in &targets {
        for (i, command) in commands.iter().enumerate() {
            if commands[..i].iter().any(|other| key(other) == key(command)) {
                return Err(SyncError::Duplicate {
                    target: target.clone(),
                    name: key(command).0.to_string(),
                });
            }
        }
    }
    Ok(targets)
}

fn for_guild(mut command: ApplicationCommand) -> ApplicationCommand {
    fn strip<const T: u8>(details: &mut CommandDetails<T>) {
        details.contexts = None;
        details.integration_types = None;
        details.dm_permission = None;
    }
    match &mut command {
        ApplicationCommand::ChatInputCommand(c) => strip(&mut c.details),
        ApplicationCommand::UserCommand(c) => strip(c),
        ApplicationCommand::MessageCommand(c) => strip(c),
    }
    command
}

/// Name and type: what identifies a Command within a target
fn key(command: &ApplicationCommand) -> (&str, u8) {
    match command {
        ApplicationCommand::ChatInputCommand(c) => (&c.details.name, 1),
        ApplicationCommand::UserCommand(c) => (&c.name, 2),
        ApplicationCommand::MessageCommand(c) => (&c.name, 3),
    }
}

fn label(command: &ApplicationCommand) -> String {
    match command {
        ApplicationCommand::ChatInputCommand(c) => format!("/{}", c.details.name),
        ApplicationCommand::UserCommand(c) => format!("{} (user)", c.name),
        ApplicationCommand::MessageCommand(c) => format!("{} (message)", c.name),
    }
}

/// One change to a target
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A Command to create, by label (`/name`, `name (user)`, `name (message)`)
    Create(String),
    /// A Command whose definition changed
    Update(String),
    /// A Command to delete
    Delete(String),
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Change::Create(label) => write!(f, "+ {label}"),
            Change::Update(label) => write!(f, "~ {label}"),
            Change::Delete(label) => write!(f, "- {label}"),
        }
    }
}

/// What a Sync would do to one target
#[derive(Debug)]
pub struct TargetPlan {
    /// Where the Commands are stored
    pub target: Target,
    /// The Commands the target ends up with
    pub commands: Vec<ApplicationCommand>,
    /// What changes
    pub changes: Vec<Change>,
    /// How many Commands stay as they are
    pub unchanged: usize,
}

/// The creates, updates, and deletes a Sync would perform, per target
#[derive(Debug)]
pub struct SyncPlan {
    /// One plan per target, global first
    pub targets: Vec<TargetPlan>,
}

impl SyncPlan {
    /// Whether applying would change anything
    pub fn has_changes(&self) -> bool {
        self.targets.iter().any(|t| !t.changes.is_empty())
    }

    /// Creates, updates, and deletes across all targets
    pub fn counts(&self) -> ChangeCounts {
        let mut counts = ChangeCounts::default();
        for change in self.targets.iter().flat_map(|t| &t.changes) {
            match change {
                Change::Create(_) => counts.create += 1,
                Change::Update(_) => counts.update += 1,
                Change::Delete(_) => counts.delete += 1,
            }
        }
        counts
    }
}

/// How many Commands a plan creates, updates and deletes
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChangeCounts {
    /// Creates
    pub create: usize,
    /// Updates
    pub update: usize,
    /// Deletes
    pub delete: usize,
}

impl ChangeCounts {
    /// All changes
    pub fn total(&self) -> usize {
        self.create + self.update + self.delete
    }
}

impl fmt::Display for SyncPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for target in &self.targets {
            if target.changes.is_empty() {
                writeln!(
                    f,
                    "{}: up to date ({} commands)",
                    target.target, target.unchanged
                )?;
                continue;
            }
            writeln!(f, "{}: {} unchanged", target.target, target.unchanged)?;
            for change in &target.changes {
                writeln!(f, "  {change}")?;
            }
        }
        Ok(())
    }
}

/// Compares what a target should have with what Discord has
pub fn plan_target(
    target: Target,
    commands: Vec<ApplicationCommand>,
    current: &[ApplicationCommand],
) -> TargetPlan {
    let mut changes = Vec::new();
    let mut unchanged = 0;

    for command in &commands {
        match current.iter().find(|c| key(c) == key(command)) {
            None => changes.push(Change::Create(label(command))),
            Some(existing) if !same(command, existing) => {
                changes.push(Change::Update(label(command)))
            }
            Some(_) => unchanged += 1,
        }
    }
    for existing in current {
        if !commands.iter().any(|c| key(c) == key(existing)) {
            changes.push(Change::Delete(label(existing)));
        }
    }

    TargetPlan {
        target,
        commands,
        changes,
        unchanged,
    }
}

/// Equal once Discord's filled-in defaults are ignored. `contexts` and `integration_types` are
/// only compared when the Definition sets them, since Discord defaults them from app settings.
fn same(desired: &ApplicationCommand, current: &ApplicationCommand) -> bool {
    let desired = normalize(serde_json::to_value(desired).unwrap_or_default());
    let mut current = normalize(serde_json::to_value(current).unwrap_or_default());
    if let (Value::Object(desired), Value::Object(current)) = (&desired, &mut current) {
        for key in ["contexts", "integration_types"] {
            if !desired.contains_key(key) {
                current.remove(key);
            }
        }
    }
    desired == current
}

fn normalize(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(key, value)| !is_default(key, value))
                .map(|(key, value)| (key, normalize(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(normalize).collect()),
        // 1 and 1.0 are the same bound
        Value::Number(n) => n.as_f64().map_or(Value::Number(n), Value::from),
        other => other,
    }
}

fn is_default(key: &str, value: &Value) -> bool {
    match (key, value) {
        (_, Value::Null) => true,
        ("guild_id", _) => true,
        ("required" | "nsfw" | "autocomplete", Value::Bool(false)) => true,
        ("dm_permission", Value::Bool(true)) => true,
        ("description", Value::String(s)) => s.is_empty(),
        ("options" | "choices" | "channel_types", Value::Array(a)) => a.is_empty(),
        ("name_localizations" | "description_localizations", Value::Object(o)) => o.is_empty(),
        _ => false,
    }
}

/// Fetches what Discord has for each target and plans the changes
pub fn plan(
    definitions: Vec<ScopedDefinition>,
    config: &SyncConfig,
) -> Result<SyncPlan, SyncError> {
    let client = client(config)?;
    let mut plans = Vec::new();
    for (target, commands) in targets(definitions, config)? {
        let current = block_on(client.get_commands(target.scope()?))?.model()?;
        plans.push(plan_target(target, commands, &current));
    }
    Ok(SyncPlan { targets: plans })
}

/// Overwrites every target in the plan that has changes
pub fn apply(plan: &SyncPlan, config: &SyncConfig) -> Result<(), SyncError> {
    let client = client(config)?;
    for target in plan.targets.iter().filter(|t| !t.changes.is_empty()) {
        apply_target(&client, target)?;
    }
    Ok(())
}

/// Bot Client for `config`; waits out rate limits up to a minute, since Sync is not under a
/// platform time limit
pub fn client(config: &SyncConfig) -> Result<BotClient<ReqwestTransport>, SyncError> {
    let application_id = parse_id("application id", &config.application_id)?;
    Ok(
        BotClient::new(ReqwestTransport::new(), &config.token, application_id)
            .with_max_rate_limit_wait(Duration::from_secs(60)),
    )
}

fn parse_id(what: &str, id: &str) -> Result<Snowflake, SyncError> {
    id.parse()
        .map_err(|_| SyncError::Config(format!("invalid {what} `{id}`")))
}

impl Target {
    fn scope(&self) -> Result<CommandScope, SyncError> {
        Ok(match self {
            Target::Global => CommandScope::Global,
            Target::Guild(guild) => CommandScope::Guild(parse_id("guild id", guild)?),
        })
    }
}

/// Overwrites one target with its planned Commands, changed or not
pub fn apply_target(
    client: &BotClient<ReqwestTransport>,
    target: &TargetPlan,
) -> Result<(), SyncError> {
    let commands: Vec<_> = target.commands.iter().collect();
    block_on(client.overwrite_commands(target.target.scope()?, &commands))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use composure::command::CommandMeta;
    use serde_json::json;

    use super::*;

    fn command(name: &str, description: &str) -> ApplicationCommand {
        CommandMeta::new(leak(name), leak(description)).chat_input(Vec::new())
    }

    fn leak(s: &str) -> &'static str {
        Box::leak(s.to_string().into_boxed_str())
    }

    fn scoped(scope: Option<&'static str>, command: ApplicationCommand) -> ScopedDefinition {
        ScopedDefinition { scope, command }
    }

    fn config(scopes: &[(&str, &[&str])]) -> SyncConfig {
        SyncConfig {
            token: "t".into(),
            application_id: "1".into(),
            scopes: scopes
                .iter()
                .map(|(k, v)| (scope_key(k), v.iter().map(|s| s.to_string()).collect()))
                .collect(),
            override_guild: None,
        }
    }

    fn names(commands: &[ApplicationCommand]) -> Vec<&str> {
        commands.iter().map(|c| key(c).0).collect()
    }

    #[test]
    fn scopes_map_to_guilds() {
        let definitions = vec![
            scoped(None, command("ping", "Ping")),
            scoped(Some("dev"), command("debug", "Debug")),
        ];
        let targets = targets(
            definitions,
            &config(&[("dev", &["1", "2"]), ("old", &["3"])]),
        )
        .unwrap();

        assert_eq!(names(&targets[&Target::Global]), ["ping"]);
        assert_eq!(names(&targets[&Target::Guild("1".into())]), ["debug"]);
        assert_eq!(names(&targets[&Target::Guild("2".into())]), ["debug"]);
        // Mapped but unused: synced empty, which clears it
        assert!(targets[&Target::Guild("3".into())].is_empty());
    }

    #[test]
    fn unmapped_scope_is_an_error() {
        let definitions = vec![scoped(Some("dev-eu"), command("debug", "Debug"))];
        let error = targets(definitions, &config(&[])).unwrap_err();
        assert_eq!(
            error.to_string(),
            "scope `dev-eu` has no guilds: set COMPOSURE_SCOPE_DEV_EU=<guild id>"
        );
    }

    #[test]
    fn override_guild_gets_everything() {
        let definitions = vec![
            scoped(None, command("ping", "Ping")),
            scoped(Some("dev"), command("debug", "Debug")),
        ];
        let mut config = config(&[]);
        config.override_guild = Some("9".into());
        let targets = targets(definitions, &config).unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(
            names(&targets[&Target::Guild("9".into())]),
            ["ping", "debug"]
        );
    }

    #[test]
    fn duplicates_are_errors() {
        let definitions = vec![
            scoped(None, command("ping", "Ping")),
            scoped(None, command("ping", "Other")),
        ];
        assert!(matches!(
            targets(definitions, &config(&[])),
            Err(SyncError::Duplicate { .. })
        ));
    }

    #[test]
    fn guild_targets_drop_global_only_fields() {
        use composure::types::{ApplicationIntegrationType, InteractionContextType};
        let mut meta = CommandMeta::new("ping", "Ping");
        meta.contexts = Some(&[InteractionContextType::Guild]);
        meta.integration_types = Some(&[ApplicationIntegrationType::GuildInstall]);
        let definitions = vec![scoped(Some("dev"), meta.chat_input(Vec::new()))];
        let targets = targets(definitions, &config(&[("dev", &["1"])])).unwrap();
        let value = serde_json::to_value(&targets[&Target::Guild("1".into())][0]).unwrap();
        assert!(value.get("contexts").is_none());
        assert!(value.get("integration_types").is_none());
    }

    fn current(value: Value) -> Vec<ApplicationCommand> {
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn plan_creates_updates_and_deletes() {
        let existing = current(json!([
            { "id": "1", "application_id": "1", "version": "1", "type": 1, "name": "ping", "description": "Ping",
              "default_member_permissions": null, "dm_permission": true, "nsfw": false, "contexts": [0, 1, 2],
              "integration_types": [0] },
            { "id": "2", "application_id": "1", "version": "1", "type": 1, "name": "echo", "description": "Old" },
            { "id": "3", "application_id": "1", "version": "1", "type": 2, "name": "Inspect", "description": "" },
        ]));
        let commands = vec![
            command("ping", "Ping"),
            command("echo", "New"),
            command("help", "Help"),
        ];
        let plan = plan_target(Target::Global, commands, &existing);

        assert_eq!(plan.unchanged, 1);
        assert_eq!(
            plan.changes,
            [
                Change::Update("/echo".into()),
                Change::Create("/help".into()),
                Change::Delete("Inspect (user)".into()),
            ]
        );
    }

    #[test]
    fn plan_ignores_filled_in_option_defaults() {
        use composure::command::{OptionMeta, OptionType};
        let mut meta = OptionMeta::new("times", "How many");
        meta.min_value = Some(1.0);
        let options = vec![
            <Option<u8> as OptionType>::definition(&meta, false),
            <String as OptionType>::definition(&OptionMeta::new("text", "Text"), true),
        ];
        let desired = CommandMeta::new("echo", "Echo").chat_input(options);
        let existing = current(json!([
            { "id": "1", "application_id": "1", "version": "1", "type": 1, "name": "echo", "description": "Echo",
              "options": [
                { "type": 4, "name": "times", "description": "How many", "min_value": 1, "max_value": 255,
                  "name_localizations": {}, "description_localizations": {} },
                { "type": 3, "name": "text", "description": "Text", "required": true, "autocomplete": false },
              ] },
        ]));
        let plan = plan_target(Target::Global, vec![desired], &existing);
        assert_eq!(plan.changes, []);
        assert_eq!(plan.unchanged, 1);
    }

    #[test]
    fn config_from_vars() {
        let config = SyncConfig::from_vars([
            ("DISCORD_TOKEN".to_string(), "t".to_string()),
            ("DISCORD_APPLICATION_ID".to_string(), "1".to_string()),
            (
                "COMPOSURE_SCOPE_DEV".to_string(),
                "798662131062931547, 798662131062931548".to_string(),
            ),
            ("PATH".to_string(), "x".to_string()),
        ])
        .unwrap();
        assert_eq!(
            config.scopes["DEV"],
            ["798662131062931547", "798662131062931548"]
        );

        let missing = SyncConfig::from_vars([("DISCORD_TOKEN".to_string(), "t".to_string())]);
        assert!(matches!(missing, Err(SyncError::Config(_))));

        let bad = SyncConfig::from_vars([
            ("DISCORD_TOKEN".to_string(), "t".to_string()),
            ("DISCORD_APPLICATION_ID".to_string(), "1".to_string()),
            ("COMPOSURE_SCOPE_DEV".to_string(), "abc".to_string()),
        ]);
        assert!(matches!(bad, Err(SyncError::Config(_))));
    }
}
