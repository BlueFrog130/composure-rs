//! Sync: make Discord's stored Command Definitions match a Command Set.
//!
//! Each target (global, or one guild) gets every Command whose Scope maps to it. A Sync Plan is
//! computed by comparing Definitions by name and type; targets with changes are overwritten with
//! one bulk request each, targets without changes are left alone.

use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    str::FromStr,
};

use composure::{
    command::ScopedDefinition,
    definition::{ApplicationCommand, CommandDetails},
    models::Snowflake,
};
use serde_json::Value;

use composure_api::DiscordClient;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
pub use cli::sync_from_env;

const SCOPE_PREFIX: &str = "COMPOSURE_SCOPE_";

/// Where a set of Commands is stored on Discord
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Global,
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
    pub token: String,
    pub application_id: String,

    /// Scope name to guild ids. Every listed guild is synced, even if no Command maps to it
    /// anymore (which clears its Commands).
    pub scopes: HashMap<String, Vec<String>>,

    /// Send the whole Command Set to this guild instead, ignoring Scopes and leaving global
    /// Commands alone. For testing.
    pub override_guild: Option<String>,
}

impl SyncConfig {
    /// `DISCORD_TOKEN`, `DISCORD_APPLICATION_ID`, and one `COMPOSURE_SCOPE_<NAME>=id[,id]` per
    /// Scope. `<NAME>` is the Scope uppercased with `-` as `_`.
    pub fn from_env() -> Result<Self, SyncError> {
        Self::from_vars(std::env::vars())
    }

    pub fn from_vars(vars: impl IntoIterator<Item = (String, String)>) -> Result<Self, SyncError> {
        let mut config = SyncConfig::default();
        for (key, value) in vars {
            if key == "DISCORD_TOKEN" {
                config.token = value;
            } else if key == "DISCORD_APPLICATION_ID" {
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
            return Err(SyncError::Config("DISCORD_TOKEN is not set".into()));
        }
        if config.application_id.is_empty() {
            return Err(SyncError::Config(
                "DISCORD_APPLICATION_ID is not set".into(),
            ));
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

#[derive(Debug)]
pub enum SyncError {
    /// Missing or malformed configuration
    Config(String),
    /// A Command's Scope has no guilds configured
    UnmappedScope(String),
    /// Two Commands with the same name and type go to the same target
    Duplicate {
        target: Target,
        name: String,
    },
    Api(composure_api::Error),
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

impl From<composure_api::Error> for SyncError {
    fn from(error: composure_api::Error) -> Self {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Create(String),
    Update(String),
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
    pub target: Target,
    /// The Commands the target ends up with
    pub commands: Vec<ApplicationCommand>,
    pub changes: Vec<Change>,
    pub unchanged: usize,
}

/// The creates, updates, and deletes a Sync would perform, per target
#[derive(Debug)]
pub struct SyncPlan {
    pub targets: Vec<TargetPlan>,
}

impl SyncPlan {
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChangeCounts {
    pub create: usize,
    pub update: usize,
    pub delete: usize,
}

impl ChangeCounts {
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
    let client = DiscordClient::new(&config.token, &config.application_id)?;
    let mut plans = Vec::new();
    for (target, commands) in targets(definitions, config)? {
        let current = match &target {
            Target::Global => client.get_global_commands()?,
            Target::Guild(guild) => client.get_guild_commands(guild)?,
        };
        plans.push(plan_target(target, commands, &current));
    }
    Ok(SyncPlan { targets: plans })
}

/// Overwrites every target in the plan that has changes
pub fn apply(plan: &SyncPlan, config: &SyncConfig) -> Result<(), SyncError> {
    let client = DiscordClient::new(&config.token, &config.application_id)?;
    for target in plan.targets.iter().filter(|t| !t.changes.is_empty()) {
        apply_target(&client, target)?;
    }
    Ok(())
}

/// Overwrites one target with its planned Commands, changed or not
pub fn apply_target(client: &DiscordClient, target: &TargetPlan) -> Result<(), SyncError> {
    let commands = target.commands.iter().collect();
    match &target.target {
        Target::Global => client.overwrite_global_commands(&commands)?,
        Target::Guild(guild) => client.overwrite_guild_commands(guild, &commands)?,
    };
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
        use composure::models::{ApplicationIntegrationType, InteractionContextType};
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
