//! Command publishing regression suite.
//!
//! `suite_round_trips_through_models` runs offline: every command in the suite must survive
//! JSON -> model -> JSON unchanged, so a field the models drop or mis-shape fails here first.
//!
//! The rest are `#[ignore]`d because they hit the live Discord API: they publish the suite,
//! read it back, and check Discord echoes everything that was sent. They need `api/.env.test`
//! (see the crate README) and overwrite the test guild's and the app's global commands.
//! Run with `cargo test -p composure_api -- --ignored`.

use std::{
    env,
    sync::{Mutex, MutexGuard},
};

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
    _guard: MutexGuard<'static, ()>,
    client: DiscordClient,
    application_id: String,
    guild_id: String,
    token: String,
}

/// Live tests share a guild and the global command list, so they run one at a time.
fn live() -> Live {
    static LOCK: Mutex<()> = Mutex::new(());
    let guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());

    dotenvy::from_filename(".env.test").expect("api/.env.test with Discord credentials");
    let var = |name: &str| {
        env::var(name)
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| panic!("{name} missing from api/.env.test"))
    };
    let token = var("DISCORD_TOKEN");
    let application_id = var("DISCORD_APPLICATION_ID");
    let guild_id = var("DISCORD_GUILD_ID");

    Live {
        _guard: guard,
        client: DiscordClient::new(&token, &application_id).unwrap(),
        application_id,
        guild_id,
        token,
    }
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
fn guild_regression() {
    let live = live();
    let suite = guild_suite();
    let commands = to_commands(&suite);

    let published = live
        .client
        .overwrite_guild_commands(&live.guild_id, &commands.iter().collect())
        .unwrap();
    assert_echoed(&suite, &published);

    let fetched = live.client.get_guild_commands(&live.guild_id).unwrap();
    assert_echoed(&suite, &fetched);

    // Publishing the same suite again is idempotent
    let republished = live
        .client
        .overwrite_guild_commands(&live.guild_id, &commands.iter().collect())
        .unwrap();
    assert_echoed(&suite, &republished);
}

#[test]
#[ignore = "hits the live Discord API; needs api/.env.test"]
fn global_regression() {
    let live = live();
    let suite = global_suite();
    let commands = to_commands(&suite);

    let published = live
        .client
        .overwrite_global_commands(&commands.iter().collect())
        .unwrap();
    assert_echoed(&suite, &published);

    let fetched = live.client.get_global_commands().unwrap();
    assert_echoed(&suite, &fetched);
}

#[test]
#[ignore = "hits the live Discord API; needs api/.env.test"]
fn update_commands_publishes_to_guild() {
    let live = live();

    let builder = CommandsBuilder::new(
        live.application_id.parse::<Snowflake>().unwrap(),
        Some(live.guild_id.parse::<Snowflake>().unwrap()),
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

    let published = builder.update_commands(&live.token).unwrap();
    assert_echoed(&sent, &published);

    let fetched = live.client.get_guild_commands(&live.guild_id).unwrap();
    assert_echoed(&sent, &fetched);
}

#[test]
#[ignore = "hits the live Discord API; needs api/.env.test"]
fn create_guild_command_echoes() {
    let live = live();
    let sent = meta_command();
    let command: ApplicationCommand = serde_json::from_value(sent.clone()).unwrap();

    let created = live
        .client
        .create_guild_command(&live.guild_id, &command)
        .unwrap();
    assert_subset(&sent, &to_value(&created), "regress-meta");

    let fetched = live.client.get_guild_commands(&live.guild_id).unwrap();
    assert!(fetched.iter().any(|c| key(&to_value(c)) == key(&sent)));
}

#[test]
#[ignore = "hits the live Discord API; needs api/.env.test"]
fn invalid_command_is_rejected() {
    let live = live();

    // Chat input names must be lowercase with no spaces
    let invalid: ApplicationCommand = serde_json::from_value(json!({
        "type": 1,
        "name": "Not Valid",
        "description": "Invalid name"
    }))
    .unwrap();

    let result = live
        .client
        .overwrite_guild_commands(&live.guild_id, &vec![&invalid]);
    match result {
        Err(Error::UnknownResponse(body)) => assert!(body.contains("50035"), "{body}"),
        other => panic!("expected Discord validation error, got {other:?}"),
    }

    let result = live.client.create_guild_command(&live.guild_id, &invalid);
    assert!(
        matches!(result, Err(Error::UnknownResponse(_))),
        "{result:?}"
    );
}

#[test]
#[ignore = "hits the live Discord API; needs api/.env.test"]
fn invalid_token_is_unauthorized() {
    let live = live();
    let client = DiscordClient::new("not-a-token", &live.application_id).unwrap();

    assert!(matches!(
        client.get_global_commands(),
        Err(Error::Unauthorized)
    ));
    assert!(matches!(
        client.get_guild_commands(&live.guild_id),
        Err(Error::Unauthorized)
    ));
}
