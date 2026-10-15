//! Command publishing regression suite.
//!
//! Offline tests check every command in the suite survives JSON -> model -> JSON unchanged, so a
//! field the models drop or mis-shape fails without touching Discord.
//!
//! `live_regression` is `#[ignore]`d because it drives the real Discord API, in order: clear all
//! global and guild commands, publish the suites, read them back raw and through the models, then
//! exercise POST, `UpdateCommands`, and error paths, leaving the guild suite installed. It reads
//! `DISCORD_TOKEN`, `DISCORD_APPLICATION_ID`, `DISCORD_GUILD_ID` from `api/.env.test` or the
//! environment (CI). Run with `cargo test -p composure_api --test regression -- --ignored`.

use std::{collections::BTreeSet, env, thread, time::Duration};

use composure::models::{
    ApplicationIntegrationType, ChannelType, InteractionContextType, Snowflake,
};
use composure_api::{DiscordClient, Error, UpdateCommands};
use composure_commands::command::{
    ApplicationCommand, ApplicationCommandOption, CommandBuilder, CommandsBuilder,
    SubcommandCommandOption,
};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

/// Chat input command with one of every option type and every option field.
fn options_command() -> Value {
    json!({
        "type": 1,
        "name": "regress-options",
        "description": "Every option type",
        "options": [
            // Required options must come first
            {
                "type": 3,
                "name": "text",
                "description": "Required string with length bounds",
                "required": true,
                "min_length": 2,
                "max_length": 50
            },
            {
                "type": 4,
                "name": "count",
                "description": "Required integer with bounds",
                "required": true,
                "min_value": -10,
                "max_value": 10
            },
            {
                "type": 3,
                "name": "color",
                "description": "String choices",
                "choices": [
                    {
                        "name": "Red",
                        "name_localizations": { "fr": "Rouge", "de": "Rot" },
                        "value": "red"
                    },
                    { "name": "Green", "value": "green" },
                    { "name": "Blue", "value": "blue" }
                ]
            },
            {
                "type": 4,
                "name": "size",
                "description": "Integer choices",
                "choices": [
                    { "name": "Small", "value": 1 },
                    { "name": "Huge", "value": 9007199254740991_i64 }
                ]
            },
            {
                "type": 10,
                "name": "ratio",
                "description": "Number with bounds",
                "min_value": 0.5,
                "max_value": 99.5
            },
            {
                "type": 10,
                "name": "factor",
                "description": "Number choices",
                "choices": [
                    { "name": "Half", "value": 0.5 },
                    { "name": "Double", "value": 2.0 }
                ]
            },
            {
                "type": 3,
                "name": "search",
                "description": "Autocomplete string",
                "autocomplete": true
            },
            {
                "type": 4,
                "name": "rank",
                "description": "Autocomplete integer",
                "autocomplete": true
            },
            {
                "type": 10,
                "name": "weight",
                "description": "Autocomplete number",
                "autocomplete": true
            },
            { "type": 5, "name": "flag", "description": "Boolean", "required": false },
            { "type": 6, "name": "member", "description": "User" },
            {
                "type": 7,
                "name": "channel",
                "description": "Text or voice channel",
                "channel_types": [0, 2]
            },
            { "type": 8, "name": "role", "description": "Role" },
            { "type": 9, "name": "target", "description": "Mentionable" },
            { "type": 11, "name": "file", "description": "Attachment" }
        ]
    })
}

/// Subcommand groups and subcommands, with and without options.
fn subcommands_command() -> Value {
    json!({
        "type": 1,
        "name": "regress-subcommands",
        "description": "Subcommands and groups",
        "options": [
            {
                "type": 2,
                "name": "admin",
                "description": "Subcommand group",
                "options": [
                    {
                        "type": 1,
                        "name": "ban",
                        "description": "Subcommand with options",
                        "options": [
                            {
                                "type": 6,
                                "name": "user",
                                "description": "User to ban",
                                "required": true
                            },
                            {
                                "type": 3,
                                "name": "reason",
                                "description": "Reason",
                                "max_length": 200
                            },
                            {
                                "type": 4,
                                "name": "days",
                                "description": "Days of messages to delete",
                                "choices": [
                                    { "name": "None", "value": 0 },
                                    { "name": "Week", "value": 7 }
                                ]
                            }
                        ]
                    },
                    {
                        "type": 1,
                        "name": "unban",
                        "description": "Subcommand without options"
                    }
                ]
            },
            {
                "type": 1,
                "name": "ping",
                "description": "Top-level subcommand without options"
            },
            {
                "type": 1,
                "name": "echo",
                "description": "Top-level subcommand with options",
                "options": [
                    {
                        "type": 3,
                        "name": "message",
                        "description": "Message",
                        "required": true
                    },
                    {
                        "type": 7,
                        "name": "where",
                        "description": "Text, announcement, or thread",
                        "channel_types": [0, 5, 11]
                    },
                    { "type": 11, "name": "file", "description": "Attachment" }
                ]
            }
        ]
    })
}

/// Localizations, member permissions, and NSFW.
fn meta_command() -> Value {
    json!({
        "type": 1,
        "name": "regress-meta",
        "name_localizations": { "fr": "regress-meta-fr", "de": "regress-meta-de" },
        "description": "Localized, permissioned, NSFW",
        "description_localizations": { "fr": "Localisé", "de": "Lokalisiert" },
        "default_member_permissions": "32",
        "nsfw": true,
        "options": [
            {
                "type": 3,
                "name": "query",
                "name_localizations": { "fr": "requete" },
                "description": "Localized option",
                "description_localizations": { "fr": "Option localisée" }
            }
        ]
    })
}

fn user_command() -> Value {
    json!({
        "type": 2,
        "name": "Regress User",
        "default_member_permissions": "0"
    })
}

fn message_command() -> Value {
    json!({
        "type": 3,
        "name": "Regress Message",
        "name_localizations": { "fr": "Regress Message FR" }
    })
}

/// User-installable, usable in every context. Global commands only.
fn contexts_command() -> Value {
    json!({
        "type": 1,
        "name": "regress-contexts",
        "description": "User-installable, usable everywhere",
        "integration_types": [0, 1],
        "contexts": [0, 1, 2]
    })
}

/// A command built through the builder API rather than JSON.
fn builder_command() -> ApplicationCommand {
    CommandBuilder::new()
        .name("regress-builder")
        .description("Built with CommandBuilder")
        .add_subcommand(|sub| {
            sub.name("say")
                .description("Builder subcommand")
                .add_option(SubcommandCommandOption::new_string_option(
                    "words".into(),
                    "What to say".into(),
                    Some(true),
                    None,
                    Some(1),
                    Some(100),
                    None,
                ))
                .add_option(SubcommandCommandOption::new_boolean_option(
                    "loud".into(),
                    "Shout it".into(),
                    None,
                ))
        })
        .add_subcommand_group(|group| {
            group
                .name("config")
                .description("Builder subcommand group")
                .add_subcommand(|sub| {
                    sub.name("set").description("Set a value").add_option(
                        SubcommandCommandOption::new_integer_option(
                            "value".into(),
                            "Value".into(),
                            Some(true),
                            None,
                            Some(0),
                            Some(1000),
                            None,
                        ),
                    )
                })
                .add_subcommand(|sub| sub.name("reset").description("Reset the value"))
        })
        .build_chat_command()
}

/// Global-only builder command exercising contexts / integration types.
fn builder_contexts_command() -> ApplicationCommand {
    CommandBuilder::new()
        .name("regress-builder-contexts")
        .description("Builder with contexts")
        .with_integration_types(&[ApplicationIntegrationType::UserInstall])
        .with_contexts(&[
            InteractionContextType::BotDm,
            InteractionContextType::PrivateChannel,
        ])
        .add_option(ApplicationCommandOption::new_number_option(
            "amount".into(),
            "Amount".into(),
            None,
            None,
            Some(-1.5),
            Some(1.5),
            None,
        ))
        .build_chat_command()
}

fn guild_suite() -> Vec<Value> {
    vec![
        options_command(),
        subcommands_command(),
        meta_command(),
        user_command(),
        message_command(),
        to_value(&builder_command()),
    ]
}

fn global_suite() -> Vec<Value> {
    let mut suite = guild_suite();
    suite.push(contexts_command());
    suite.push(to_value(&builder_contexts_command()));
    suite
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_value(command: &ApplicationCommand) -> Value {
    serde_json::to_value(command).unwrap()
}

fn to_commands(suite: &[Value]) -> Vec<ApplicationCommand> {
    suite
        .iter()
        .map(|command| serde_json::from_value(command.clone()).unwrap())
        .collect()
}

fn key(command: &Value) -> (u64, String) {
    (
        command["type"].as_u64().unwrap(),
        command["name"].as_str().unwrap().to_string(),
    )
}

/// Values Discord omits from responses when they are the default.
fn is_default(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(false) => true,
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// Everything in `expected` is present and equal in `actual`. `actual` may have extra fields.
fn assert_subset(expected: &Value, actual: &Value, path: &str) {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            for (k, v) in expected {
                let path = format!("{path}.{k}");
                match actual.get(k) {
                    Some(a) => assert_subset(v, a, &path),
                    None => assert!(is_default(v), "{path}: sent {v}, missing in response"),
                }
            }
        }
        (Value::Array(expected), Value::Array(actual)) => {
            assert_eq!(
                expected.len(),
                actual.len(),
                "{path}: sent {expected:?}, got {actual:?}"
            );
            // Scalar lists (channel_types, contexts, ...) are sets; Discord may reorder them
            if expected.iter().all(|v| !v.is_object()) {
                let sorted = |values: &[Value]| {
                    let mut values: Vec<String> = values.iter().map(Value::to_string).collect();
                    values.sort();
                    values
                };
                assert_eq!(
                    sorted(expected),
                    sorted(actual),
                    "{path}: sent {expected:?}, got {actual:?}"
                );
                return;
            }
            for (i, (e, a)) in expected.iter().zip(actual).enumerate() {
                assert_subset(e, a, &format!("{path}[{i}]"));
            }
        }
        (Value::Number(e), Value::Number(a)) => {
            assert_eq!(e.as_f64(), a.as_f64(), "{path}: sent {e}, got {a}");
        }
        (Value::Null, Value::Array(a)) if a.is_empty() => {}
        _ => assert_eq!(expected, actual, "{path}"),
    }
}

/// `returned` is exactly the commands in `sent`, each echoing everything that was sent.
fn assert_echoed(sent: &[Value], returned: &[ApplicationCommand]) {
    let returned: Vec<Value> = returned.iter().map(to_value).collect();

    let mut sent_keys: Vec<_> = sent.iter().map(key).collect();
    let mut returned_keys: Vec<_> = returned.iter().map(key).collect();
    sent_keys.sort();
    returned_keys.sort();
    assert_eq!(sent_keys, returned_keys);

    for command in sent {
        let actual = returned.iter().find(|r| key(r) == key(command)).unwrap();
        assert_subset(command, actual, &command["name"].to_string());
    }
}

struct Live {
    client: DiscordClient,
    http: reqwest::blocking::Client,
    application_id: String,
    guild_id: String,
    token: String,
}

impl Live {
    /// Credentials from `api/.env.test` if present, otherwise the environment.
    fn new() -> Self {
        let _ = dotenvy::from_filename(".env.test");
        let var = |name: &str| {
            env::var(name)
                .ok()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| panic!("{name} missing from api/.env.test and the environment"))
        };
        let token = var("DISCORD_TOKEN");
        let application_id = var("DISCORD_APPLICATION_ID");
        let guild_id = var("DISCORD_GUILD_ID");

        Live {
            client: DiscordClient::new(&token, &application_id).unwrap(),
            http: reqwest::blocking::Client::new(),
            application_id,
            guild_id,
            token,
        }
    }

    fn global_url(&self) -> String {
        format!(
            "{}/applications/{}/commands?with_localizations=true",
            composure_api::DISCORD_API,
            self.application_id
        )
    }

    fn guild_url(&self) -> String {
        format!(
            "{}/applications/{}/guilds/{}/commands?with_localizations=true",
            composure_api::DISCORD_API,
            self.application_id,
            self.guild_id
        )
    }

    /// Raw JSON from Discord, bypassing the models.
    fn raw_get(&self, url: &str) -> Vec<Value> {
        for _ in 0..5 {
            let response = self
                .http
                .get(url)
                .header("Authorization", format!("Bot {}", self.token))
                .send()
                .unwrap();
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let body: Value = response.json().unwrap();
                let wait = body["retry_after"].as_f64().unwrap_or(1.0);
                thread::sleep(Duration::from_secs_f64(wait));
                continue;
            }
            assert!(
                response.status().is_success(),
                "GET {url}: {}",
                response.status()
            );
            return response.json().unwrap();
        }
        panic!("GET {url}: still rate limited");
    }
}

/// Fields Discord returns that the models intentionally don't round-trip.
const UNMODELED_FIELDS: &[&str] = &[
    "id",
    "application_id",
    "version",
    "default_permission",
    "name_localized",
    "description_localized",
    "handler",
];

/// Every raw command deserializes; the model keeps every value it models (`model ⊆ raw`) and
/// reads ids. Raw fields the model drops beyond [`UNMODELED_FIELDS`] are reported.
fn assert_deserializes(raw: &[Value]) -> Vec<ApplicationCommand> {
    raw.iter()
        .map(|raw| {
            let name = raw["name"].to_string();
            let model: ApplicationCommand = serde_json::from_value(raw.clone())
                .unwrap_or_else(|e| panic!("{name} failed to deserialize: {e}\n{raw:#}"));

            let id = match &model {
                ApplicationCommand::ChatInputCommand(c) => &c.details.id,
                ApplicationCommand::UserCommand(c) => &c.id,
                ApplicationCommand::MessageCommand(c) => &c.id,
            };
            assert!(id.is_some(), "{name}: id not deserialized");

            let modeled = to_value(&model);
            assert_subset(&modeled, raw, &name);

            let mut dropped = BTreeSet::new();
            unmodeled_paths(raw, &modeled, &name, &mut dropped);
            if !dropped.is_empty() {
                eprintln!("note: fields not modeled: {dropped:?}");
            }

            model
        })
        .collect()
}

fn unmodeled_paths(raw: &Value, modeled: &Value, path: &str, out: &mut BTreeSet<String>) {
    match (raw, modeled) {
        (Value::Object(raw), Value::Object(modeled)) => {
            for (k, v) in raw {
                let path = format!("{path}.{k}");
                match modeled.get(k) {
                    Some(m) => unmodeled_paths(v, m, &path, out),
                    None if UNMODELED_FIELDS.contains(&k.as_str()) || is_default(v) => {}
                    None => {
                        out.insert(path);
                    }
                }
            }
        }
        (Value::Array(raw), Value::Array(modeled)) => {
            for (i, (r, m)) in raw.iter().zip(modeled).enumerate() {
                unmodeled_paths(r, m, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

fn step(name: &str) {
    eprintln!("==> {name}");
}

// ---------------------------------------------------------------------------
// Offline
// ---------------------------------------------------------------------------

#[test]
fn suite_round_trips_through_models() {
    for command in global_suite() {
        let model: ApplicationCommand = serde_json::from_value(command.clone()).unwrap();
        assert_eq!(to_value(&model), command, "{}", command["name"]);
    }
}

#[test]
fn builder_contexts_serialize() {
    let command = to_value(&builder_contexts_command());
    assert_eq!(command["integration_types"], json!([1]));
    assert_eq!(command["contexts"], json!([1, 2]));
}

#[test]
fn subset_tolerates_defaults_and_extra_fields() {
    let sent = json!({ "name": "a", "required": false, "options": [{ "x": 1.0 }] });
    let got = json!({ "name": "a", "id": "1", "options": [{ "x": 1, "y": 2 }] });
    assert_subset(&sent, &got, "cmd");
}

#[test]
fn subset_ignores_scalar_list_order() {
    assert_subset(
        &json!({ "c": [0, 5, 11] }),
        &json!({ "c": [0, 11, 5] }),
        "cmd",
    );
}

#[test]
#[should_panic(expected = "cmd.c")]
fn subset_detects_scalar_list_changes() {
    assert_subset(&json!({ "c": [0, 5] }), &json!({ "c": [0, 11] }), "cmd");
}

#[test]
#[should_panic(expected = "cmd.name")]
fn subset_detects_changed_values() {
    assert_subset(&json!({ "name": "a" }), &json!({ "name": "b" }), "cmd");
}

#[test]
#[should_panic(expected = "missing in response")]
fn subset_detects_dropped_values() {
    assert_subset(&json!({ "nsfw": true }), &json!({}), "cmd");
}

#[test]
fn channel_types_model() {
    let command: ApplicationCommand = serde_json::from_value(options_command()).unwrap();
    let options = command
        .as_chat_input_command()
        .unwrap()
        .options
        .as_ref()
        .unwrap();
    let channel = options
        .iter()
        .find_map(|o| match o {
            ApplicationCommandOption::Channel(c) => Some(c),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        channel.channel_types,
        Some(vec![ChannelType::GuildText, ChannelType::GuildVoice])
    );
}

// ---------------------------------------------------------------------------
// Live
// ---------------------------------------------------------------------------

#[test]
#[ignore = "hits the live Discord API; needs api/.env.test"]
fn live_regression() {
    let live = Live::new();
    let guild = live.guild_id.as_str();
    let none: Vec<&ApplicationCommand> = Vec::new();

    step("clear global commands");
    assert!(
        live.client
            .overwrite_global_commands(&none)
            .unwrap()
            .is_empty()
    );
    assert!(live.raw_get(&live.global_url()).is_empty());

    step("clear guild commands");
    assert!(
        live.client
            .overwrite_guild_commands(guild, &none)
            .unwrap()
            .is_empty()
    );
    assert!(live.raw_get(&live.guild_url()).is_empty());

    step("publish global suite");
    let suite = global_suite();
    let commands = to_commands(&suite);
    let published = live
        .client
        .overwrite_global_commands(&commands.iter().collect())
        .unwrap();
    assert_echoed(&suite, &published);

    step("read back global suite");
    assert_echoed(
        &suite,
        &assert_deserializes(&live.raw_get(&live.global_url())),
    );
    assert_echoed(&suite, &live.client.get_global_commands().unwrap());

    step("publish guild suite");
    let suite = guild_suite();
    let commands = to_commands(&suite);
    let published = live
        .client
        .overwrite_guild_commands(guild, &commands.iter().collect())
        .unwrap();
    assert_echoed(&suite, &published);

    step("read back guild suite");
    assert_echoed(
        &suite,
        &assert_deserializes(&live.raw_get(&live.guild_url())),
    );
    assert_echoed(&suite, &live.client.get_guild_commands(guild).unwrap());

    step("republish guild suite (idempotent)");
    let republished = live
        .client
        .overwrite_guild_commands(guild, &commands.iter().collect())
        .unwrap();
    assert_echoed(&suite, &republished);

    step("create one guild command via POST");
    let mut created_json = meta_command();
    created_json["name"] = json!("regress-create");
    created_json["name_localizations"] = json!({ "fr": "regress-create-fr" });
    let created: ApplicationCommand = serde_json::from_value(created_json.clone()).unwrap();
    let created = live.client.create_guild_command(guild, &created).unwrap();
    assert_subset(&created_json, &to_value(&created), "regress-create");
    let mut with_created = suite.clone();
    with_created.push(created_json);
    assert_echoed(
        &with_created,
        &assert_deserializes(&live.raw_get(&live.guild_url())),
    );

    step("publish via UpdateCommands");
    let builder = CommandsBuilder::new(
        live.application_id.parse::<Snowflake>().unwrap(),
        Some(guild.parse::<Snowflake>().unwrap()),
    )
    .add_command(|c| {
        c.name("regress-update")
            .description("Published via UpdateCommands")
    })
    .add_command(|c| {
        c.name("regress-update-sub")
            .description("Published via UpdateCommands")
            .add_subcommand(|s| s.name("go").description("Go"))
    });
    let sent: Vec<Value> = builder.commands.iter().map(to_value).collect();
    assert_echoed(&sent, &builder.update_commands(&live.token).unwrap());
    assert_echoed(
        &sent,
        &assert_deserializes(&live.raw_get(&live.guild_url())),
    );

    step("invalid command is rejected");
    // Chat input names must be lowercase with no spaces
    let invalid: ApplicationCommand = serde_json::from_value(json!({
        "type": 1,
        "name": "Not Valid",
        "description": "Invalid name"
    }))
    .unwrap();
    match live.client.overwrite_guild_commands(guild, &vec![&invalid]) {
        Err(Error::UnknownResponse(body)) => assert!(body.contains("50035"), "{body}"),
        other => panic!("expected Discord validation error, got {other:?}"),
    }
    let result = live.client.create_guild_command(guild, &invalid);
    assert!(
        matches!(result, Err(Error::UnknownResponse(_))),
        "{result:?}"
    );

    step("invalid token is unauthorized");
    let unauthorized = DiscordClient::new("not-a-token", &live.application_id).unwrap();
    assert!(matches!(
        unauthorized.get_global_commands(),
        Err(Error::Unauthorized)
    ));
    assert!(matches!(
        unauthorized.get_guild_commands(guild),
        Err(Error::Unauthorized)
    ));

    step("restore guild suite");
    let restored = live
        .client
        .overwrite_guild_commands(guild, &commands.iter().collect())
        .unwrap();
    assert_echoed(&suite, &restored);
}
