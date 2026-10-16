//! Signed interactions through `handle_interaction` on the host, and the Definitions Sync would
//! publish. `greet` needs the Workers `Env`, so it is only checked for its Definition.

use composure::{
    command::{CommandSetDefinition, InteractionHandler},
    commands,
};
use composure_adapter_cloudflare::handle_interaction;
use composure_example_commands::{
    Bot, Components, Mod, avatar, echo, hello, ice_cream, word_count,
};
use ed25519_dalek::{Signer, SigningKey};
use futures::executor::block_on;
use serde_json::{Value, json};

// The same commands minus `greet`, which needs `worker::Env` as State
commands! {
    struct HostBot = [hello, echo, ice_cream, Mod, avatar, word_count];
}

const KEY: [u8; 32] = [7; 32];
const TIMESTAMP: &str = "1760000000";
const ALICE: &str = "100000000000000001";

fn alice() -> Value {
    json!({ "id": ALICE, "username": "alice", "discriminator": "0", "global_name": "Alice", "avatar": "abc" })
}

fn send(handler: &impl InteractionHandler<()>, payload: Value) -> (u16, Value) {
    let body = payload.to_string().into_bytes();
    let key = SigningKey::from_bytes(&KEY);
    let signature = hex::encode(key.sign(&[TIMESTAMP.as_bytes(), &body].concat()).to_bytes());
    let outcome = block_on(handle_interaction(
        &hex::encode(key.verifying_key().as_bytes()),
        Some(&signature),
        Some(TIMESTAMP),
        &body,
        handler,
        &(),
    ));
    let response = outcome
        .response()
        .map(|r| serde_json::to_value(r).unwrap())
        .unwrap_or_default();
    (outcome.status(), response)
}

fn command(data: Value) -> Value {
    json!({
        "id": "1100173248714518568",
        "application_id": "1052322265397739523",
        "type": 2,
        "token": "token",
        "version": 1,
        "user": alice(),
        "data": data,
    })
}

fn content(payload: Value) -> String {
    let (status, response) = send(&HostBot, payload);
    assert_eq!(status, 200, "{response}");
    response["data"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn chat_input_commands() {
    assert_eq!(
        content(command(json!({ "id": "1", "name": "hello", "type": 1 }))),
        "Hello, Alice!"
    );
    assert_eq!(
        content(command(
            json!({ "id": "1", "name": "echo", "type": 1, "options": [
            { "name": "text", "type": 3, "value": "hi" },
            { "name": "times", "type": 4, "value": 3 },
        ] })
        )),
        "hi hi hi"
    );

    let (_, response) = send(
        &HostBot,
        command(
            json!({ "id": "1", "name": "ice-cream", "type": 1, "options": [
            { "name": "flavor", "type": 3, "value": "mint" },
        ] }),
        ),
    );
    assert_eq!(response["data"]["embeds"][0]["title"], "1 x MintChip");
}

#[test]
fn group_and_context_menu_commands() {
    let ban = command(json!({
        "id": "1", "name": "mod", "type": 1,
        "options": [{ "name": "ban", "type": 1, "options": [{ "name": "user", "type": 6, "value": ALICE }] }],
        "resolved": { "users": { ALICE: alice() } },
    }));
    assert_eq!(content(ban), "Would ban Alice (no reason)");

    let avatar = command(json!({
        "id": "1", "name": "Avatar", "type": 2, "target_id": ALICE,
        "resolved": { "users": { ALICE: alice() } },
    }));
    assert_eq!(
        content(avatar),
        format!("https://cdn.discordapp.com/avatars/{ALICE}/abc.png")
    );
}

#[test]
fn errors_become_ephemeral_messages() {
    let (status, response) = send(
        &HostBot,
        command(json!({ "id": "1", "name": "nope", "type": 1 })),
    );
    assert_eq!(status, 200);
    assert_eq!(response["data"]["content"], "Unknown command.");
    assert_eq!(response["data"]["flags"], 64);

    // `text` is required
    let (_, response) = send(
        &HostBot,
        command(json!({ "id": "1", "name": "echo", "type": 1 })),
    );
    assert_eq!(response["data"]["flags"], 64);
}

#[test]
fn fallback_handles_components() {
    let click = json!({
        "id": "1", "application_id": "1", "type": 3, "token": "t", "version": 1,
        "data": { "custom_id": "button", "component_type": 2 },
    });
    let (_, response) = send(&HostBot.with_fallback(Components), click);
    assert_eq!(response["data"]["content"], "Clicked button");
}

#[test]
fn definitions_for_sync() {
    let definitions = Bot::definitions();
    let summary: Vec<(Option<&str>, Value, Value)> = definitions
        .iter()
        .map(|d| {
            let value = serde_json::to_value(&d.command).unwrap();
            (d.scope, value["name"].clone(), value["type"].clone())
        })
        .collect();
    assert_eq!(
        summary,
        [
            (None, json!("hello"), json!(1)),
            (None, json!("echo"), json!(1)),
            (None, json!("ice-cream"), json!(1)),
            (None, json!("greet"), json!(1)),
            (Some("dev"), json!("mod"), json!(1)),
            (None, json!("Avatar"), json!(2)),
            (None, json!("Word count"), json!(3)),
        ]
    );
}
