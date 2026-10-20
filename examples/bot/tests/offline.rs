//! Signed interactions through `handle_interaction` on the host, and the Definitions Sync would
//! publish. `TestEnv` stands in for an Adapter's State (`Worker`, `Host`): vars, a recording Transport,
//! and Background Work run on demand.

use std::{
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

use composure::adapter::{handle_event, handle_interaction};
use composure::{
    command::{
        BackgroundWork, CommandSetDefinition, HandlerError, HasData, InteractionHandler, Platform,
    },
    rest::{Method, RawResponse, Request, Transport, TransportError},
};
use composure_example_bot::{Bot, Config, THEME};
use ed25519_dalek::{Signer, SigningKey};
use futures::executor::block_on;
use serde_json::{Value, json};

/// Records requests and answers the few that are read with canned bodies
#[derive(Default)]
struct Discord {
    requests: Mutex<Vec<Request>>,
}

impl Transport for Discord {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        self.requests.lock().unwrap().push(request.clone());
        let path = request.url.rsplit("/v10").next().unwrap();
        let body = match (request.method, path) {
            (Method::Get, "/users/@me") => json!({
                "id": "9", "username": "composure", "discriminator": "0",
                "global_name": "Composure", "avatar": null,
            }),
            (Method::Post, "/users/@me/channels") => json!({ "id": "77", "type": 1 }),
            (Method::Get, p) if p.starts_with("/applications/") => json!([
                { "id": "1", "application_id": "1", "version": "1", "type": 1, "name": "hello", "description": "Say hello" },
                { "id": "2", "application_id": "1", "version": "1", "type": 2, "name": "Avatar" },
            ]),
            _ => json!({}),
        };
        Ok(RawResponse {
            status: 200,
            headers: Vec::new(),
            body: body.to_string().into_bytes(),
        })
    }

    async fn sleep(&self, _duration: Duration) {}
}

#[derive(Clone, Default)]
struct TestEnv {
    discord: Arc<Discord>,
    work: Arc<Mutex<Vec<BackgroundWork>>>,
}

static CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    greeting: "Howdy".into(),
});

impl HasData for TestEnv {
    type Data = Config;

    fn data(&self) -> &Config {
        &CONFIG
    }
}

impl Platform for TestEnv {
    type Transport = Arc<Discord>;

    fn transport(&self) -> Arc<Discord> {
        self.discord.clone()
    }

    fn spawn(&self, work: BackgroundWork) {
        self.work.lock().unwrap().push(work);
    }

    fn bot_token(&self) -> Option<String> {
        Some("bot-token".to_string())
    }
}

impl TestEnv {
    /// Runs the Background Work spawned so far, as `wait_until` would after the response
    fn run_background(&self) -> Result<(), HandlerError> {
        let work: Vec<_> = self.work.lock().unwrap().drain(..).collect();
        work.into_iter().try_for_each(block_on)
    }

    /// (method, path, content type, body) of every request sent
    fn sent(&self) -> Vec<(Method, String, String, String)> {
        self.discord
            .requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| {
                (
                    r.method,
                    r.url.rsplit("/v10").next().unwrap().to_string(),
                    r.header("content-type").unwrap_or_default().to_string(),
                    String::from_utf8_lossy(r.body.as_deref().unwrap_or_default()).into_owned(),
                )
            })
            .collect()
    }
}

const KEY: [u8; 32] = [7; 32];
const TIMESTAMP: &str = "1760000000";
const ALICE: &str = "100000000000000001";
const CHANNEL: &str = "300000000000000001";

fn alice() -> Value {
    json!({ "id": ALICE, "username": "alice", "discriminator": "0", "global_name": "Alice", "avatar": "abc" })
}

fn send_with(
    handler: &impl InteractionHandler<TestEnv>,
    env: &TestEnv,
    payload: Value,
) -> (u16, Value, usize) {
    let body = payload.to_string().into_bytes();
    let key = SigningKey::from_bytes(&KEY);
    let signature = hex::encode(key.sign(&[TIMESTAMP.as_bytes(), &body].concat()).to_bytes());
    let outcome = block_on(handle_interaction(
        &hex::encode(key.verifying_key().as_bytes()),
        Some(&signature),
        Some(TIMESTAMP),
        &body,
        handler,
        env,
    ));
    let files = outcome.response().map_or(0, |r| r.files().len());
    let response = outcome
        .response()
        .map(|r| serde_json::to_value(r).unwrap())
        .unwrap_or_default();
    (outcome.status(), response, files)
}

fn send(handler: &impl InteractionHandler<TestEnv>, payload: Value) -> (u16, Value) {
    let (status, response, _) = send_with(handler, &TestEnv::default(), payload);
    (status, response)
}

fn command(data: Value) -> Value {
    json!({
        "id": "1100173248714518568",
        "application_id": "1052322265397739523",
        "type": 2,
        "token": "token",
        "version": 1,
        "channel_id": CHANNEL,
        "user": alice(),
        "data": data,
    })
}

fn content(payload: Value) -> String {
    let (status, response) = send(&Bot, payload);
    assert_eq!(status, 200, "{response}");
    response["data"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

const ORIGINAL: &str = "/webhooks/1052322265397739523/token/messages/@original";

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
        &Bot,
        command(
            json!({ "id": "1", "name": "ice-cream", "type": 1, "options": [
            { "name": "flavor", "type": 3, "value": "mint" },
        ] }),
        ),
    );
    let embed = &response["data"]["embeds"][0];
    assert_eq!(embed["title"], "1 x Mint chip");
    assert_eq!(embed["color"], THEME.primary);
    assert_eq!(embed["footer"]["text"], "composure example");

    assert_eq!(
        content(command(json!({ "id": "1", "name": "greet", "type": 1 }))),
        "Howdy from Composure!"
    );
}

#[test]
fn group_and_context_menu_commands() {
    let ban = command(json!({
        "id": "1", "name": "mod", "type": 1,
        "options": [{ "name": "ban", "type": 1, "options": [{ "name": "user", "type": 6, "value": ALICE }] }],
        "resolved": { "users": { ALICE: alice() } },
    }));
    let (_, response) = send(&Bot, ban);
    let error = &response["data"]["embeds"][0];
    assert_eq!(error["description"], "You can't ban yourself.");
    assert_eq!(error["color"], THEME.error);
    assert_eq!(error["footer"]["text"], "composure example");
    assert_eq!(response["data"]["flags"], 64);

    let bob = json!({ "id": "100000000000000002", "username": "bob", "discriminator": "0", "global_name": "Bob" });
    let ban = command(json!({
        "id": "1", "name": "mod", "type": 1,
        "options": [{ "name": "ban", "type": 1, "options": [
            { "name": "user", "type": 6, "value": "100000000000000002" },
            { "name": "reason", "type": 3, "value": "spam" },
        ] }],
        "resolved": { "users": { "100000000000000002": bob } },
    }));
    let (_, response) = send(&Bot, ban);
    let embed = &response["data"]["embeds"][0];
    assert_eq!(embed["description"], "Would ban Bob (spam)");
    assert_eq!(embed["color"], THEME.warning);
    assert_eq!(response["data"]["flags"], 64);

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
fn inline_rest_with_the_bot_token() {
    let env = TestEnv::default();
    let (_, response, _) = send_with(
        &Bot,
        &env,
        command(json!({ "id": "1", "name": "whoami", "type": 1 })),
    );
    assert_eq!(response["data"]["content"], "I'm Composure.");
    let request = env.discord.requests.lock().unwrap()[0].clone();
    assert_eq!(request.header("authorization"), Some("Bot bot-token"));

    let env = TestEnv::default();
    let (_, response, _) = send_with(
        &Bot,
        &env,
        command(
            json!({ "id": "1", "name": "announce", "type": 1, "options": [
            { "name": "text", "type": 3, "value": "Release day" },
        ] }),
        ),
    );
    assert_eq!(response["data"]["content"], "Posted.");
    assert_eq!(
        env.sent(),
        [(
            Method::Post,
            format!("/channels/{CHANNEL}/messages"),
            "application/json".to_string(),
            r#"{"content":"Release day"}"#.to_string()
        )]
    );

    let env = TestEnv::default();
    let (_, response, _) = send_with(
        &Bot,
        &env,
        command(json!({ "id": "1", "name": "dm-me", "type": 1, "options": [
            { "name": "text", "type": 3, "value": "psst" },
        ] })),
    );
    assert_eq!(response["data"]["content"], "Check your DMs.");
    let paths: Vec<_> = env.sent().into_iter().map(|(_, path, ..)| path).collect();
    assert_eq!(paths, ["/users/@me/channels", "/channels/77/messages"]);
}

#[test]
fn message_command_reacts() {
    let env = TestEnv::default();
    let message = json!({
        "id": "500000000000000001", "channel_id": CHANNEL, "author": alice(), "content": "nice",
        "timestamp": "2024-01-01T00:00:00.000000+00:00", "edited_timestamp": null, "tts": false,
        "mention_everyone": false, "mentions": [], "mention_roles": [], "attachments": [],
        "embeds": [], "pinned": false, "type": 0,
    });
    let (_, response, _) = send_with(
        &Bot,
        &env,
        command(json!({
            "id": "1", "name": "Thumbs up", "type": 3, "target_id": "500000000000000001",
            "resolved": { "messages": { "500000000000000001": message } },
        })),
    );
    assert_eq!(response["data"]["content"], "Reacted.");
    assert_eq!(
        env.sent()[0].1,
        format!("/channels/{CHANNEL}/messages/500000000000000001/reactions/%F0%9F%91%8D/@me")
    );
}

#[test]
fn deferred_commands_finish_in_the_background() {
    let env = TestEnv::default();
    let (_, response, _) = send_with(
        &Bot,
        &env,
        command(json!({ "id": "1", "name": "remind", "type": 1, "options": [
            { "name": "text", "type": 3, "value": "stretch" },
            { "name": "seconds", "type": 4, "value": 2 },
        ] })),
    );
    // Answered right away; nothing sent to Discord until the response is out
    assert_eq!(response, json!({ "type": 5, "data": { "flags": 64 } }));
    assert!(env.sent().is_empty());

    env.run_background().unwrap();
    let sent = env.sent();
    assert_eq!(sent.len(), 2);
    assert_eq!(
        (sent[0].0, sent[0].1.as_str()),
        (Method::Post, "/webhooks/1052322265397739523/token")
    );
    assert!(sent[0].3.contains(&format!("<@{ALICE}> stretch")));
    assert_eq!((sent[1].0, sent[1].1.as_str()), (Method::Patch, ORIGINAL));
    assert_eq!(
        sent[1].3,
        r#"{"content":"Reminded you about \"stretch\"."}"#
    );

    let env = TestEnv::default();
    let (_, response, _) = send_with(
        &Bot,
        &env,
        command(json!({ "id": "1", "name": "countdown", "type": 1 })),
    );
    assert_eq!(response, json!({ "type": 5 }));
    env.run_background().unwrap();
    let edits: Vec<_> = env.sent().into_iter().map(|(.., body)| body).collect();
    assert_eq!(
        edits,
        [
            r#"{"content":"3..."}"#,
            r#"{"content":"2..."}"#,
            r#"{"content":"1..."}"#,
            "{\"content\":\"Liftoff! \u{1F680}\"}",
        ]
    );
}

#[test]
fn files_upload_as_multipart() {
    // In the initial response: the Adapter sends it through the callback endpoint
    let (_, response, files) = send_with(
        &Bot,
        &TestEnv::default(),
        command(json!({ "id": "1", "name": "export", "type": 1, "options": [
            { "name": "text", "type": 3, "value": "hello file" },
        ] })),
    );
    assert_eq!(response["data"]["content"], "Here's your file:");
    assert_eq!(files, 1);

    // In a deferred result: with the edit of the original response
    let env = TestEnv::default();
    send_with(
        &Bot,
        &env,
        command(json!({ "id": "1", "name": "report", "type": 1 })),
    );
    env.run_background().unwrap();
    let sent = env.sent();
    assert_eq!(
        sent[0].1,
        "/applications/1052322265397739523/commands?with_localizations=true"
    );
    let (method, path, content_type, body) = &sent[1];
    assert_eq!((*method, path.as_str()), (Method::Patch, ORIGINAL));
    assert!(content_type.starts_with("multipart/form-data"));
    assert!(body.contains("name,type\nhello,slash\nAvatar,user\n"));
    assert!(body.contains(r#""content":"2 global commands""#));
}

#[test]
fn errors_become_ephemeral_themed_embeds() {
    let (status, response) = send(
        &Bot,
        command(json!({ "id": "1", "name": "nope", "type": 1 })),
    );
    assert_eq!(status, 200);
    let error = &response["data"]["embeds"][0];
    assert_eq!(error["description"], "Unknown command.");
    assert_eq!(error["color"], THEME.error);
    assert_eq!(response["data"]["flags"], 64);

    // `text` is required
    let (_, response) = send(
        &Bot,
        command(json!({ "id": "1", "name": "echo", "type": 1 })),
    );
    assert_eq!(response["data"]["flags"], 64);
}

fn click(custom_id: &str, values: &[&str]) -> Value {
    json!({
        "id": "1100173248714518568", "application_id": "1052322265397739523", "type": 3,
        "token": "token", "version": 1, "user": alice(),
        "data": { "custom_id": custom_id, "component_type": 2, "values": values },
    })
}

fn custom_id(response: &Value) -> &str {
    response["data"]["components"][0]["components"][0]["custom_id"]
        .as_str()
        .unwrap()
}

#[test]
fn buttons_carry_state_in_the_custom_id() {
    let (_, response) = send(
        &Bot,
        command(json!({ "id": "1", "name": "counter", "type": 1 })),
    );
    assert_eq!(response["data"]["content"], "Clicked 0 times");
    assert_eq!(custom_id(&response), "counter:0");

    let (_, response) = send(&Bot, click("counter:4", &[]));
    assert_eq!(response["type"], 7, "updates the clicked message");
    assert_eq!(response["data"]["content"], "Clicked 5 times");
    assert_eq!(custom_id(&response), "counter:5");

    // No handler for the prefix: the default "not handled" message
    let (_, response) = send(&Bot, click("button", &[]));
    assert_eq!(
        response["data"]["content"],
        "This bot does not handle components."
    );
}

#[test]
fn components_v2_card_is_themed_and_updates_in_place() {
    let (_, response) = send(
        &Bot,
        command(json!({ "id": "1", "name": "card", "type": 1 })),
    );
    let data = &response["data"];
    assert_eq!(data["flags"], 1 << 15, "IS_COMPONENTS_V2");
    assert_eq!(data.get("content"), None);
    let container = &data["components"][0];
    assert_eq!(container["type"], 17);
    assert_eq!(container["accent_color"], THEME.primary);
    assert_eq!(container["components"][0]["accessory"]["style"], 5);
    assert_eq!(container["components"][2]["content"], "-# 0 likes");
    assert_eq!(
        container["components"][3]["components"][0]["custom_id"],
        "card:like:0"
    );

    let (_, response) = send(&Bot, click("card:like:2", &[]));
    assert_eq!(response["type"], 7);
    let container = &response["data"]["components"][0];
    assert_eq!(container["components"][2]["content"], "-# 3 likes");
    assert_eq!(container["accent_color"], THEME.primary);
    assert_eq!(
        container["components"][3]["components"][1]["custom_id"],
        "card:review:3"
    );
}

#[test]
fn view_opens_a_modal_view_and_reads_its_fields() {
    let (_, response) = send(&Bot, click("card:review:3", &[]));
    assert_eq!(response["type"], 9);
    let data = &response["data"];
    assert_eq!(data["custom_id"], "review:send:3");
    assert_eq!(
        data["components"][0]["component"]["options"][1]["label"],
        "★★"
    );
    assert_eq!(data["components"][1]["component"]["required"], false);

    let label = |child: Value| json!({ "type": 18, "id": 1, "component": child });
    let submit = json!({
        "id": "1", "application_id": "1", "type": 5, "token": "t", "version": 1,
        "data": { "custom_id": "review:send:3", "components": [
            label(json!({ "type": 21, "custom_id": "stars", "value": "Three" })),
            label(json!({ "type": 4, "custom_id": "comment", "value": "neat" })),
        ] },
    });
    let (_, response) = send(&Bot, submit);
    assert_eq!(
        response["data"]["content"],
        "★★★ for a card with 3 likes: neat"
    );
    assert_eq!(response["data"]["flags"], 64);
}

#[test]
fn view_handler_defers_and_rerenders_later() {
    let env = TestEnv::default();
    let (_, response, _) = send_with(&Bot, &env, click("dice:roll:2", &[]));
    assert_eq!(response, json!({ "type": 6 }));

    env.run_background().unwrap();
    let sent = env.sent();
    assert_eq!((sent[0].0, sent[0].1.as_str()), (Method::Patch, ORIGINAL));
    let body: Value = serde_json::from_str(&sent[0].3).unwrap();
    assert_eq!(body["flags"], 1 << 15, "{body}");
    assert!(
        body["components"][0]["content"]
            .as_str()
            .unwrap()
            .ends_with("(3 rolls)"),
        "{body}"
    );
    assert_eq!(
        body["components"][1]["components"][0]["custom_id"],
        "dice:roll:3"
    );
}

#[test]
fn deferred_update_edits_the_message_later() {
    let env = TestEnv::default();
    let (_, response, _) = send_with(&Bot, &env, click("roll:2", &[]));
    assert_eq!(response, json!({ "type": 6 }));
    assert!(env.sent().is_empty());

    env.run_background().unwrap();
    let sent = env.sent();
    assert_eq!((sent[0].0, sent[0].1.as_str()), (Method::Patch, ORIGINAL));
    let body: Value = serde_json::from_str(&sent[0].3).unwrap();
    assert!(
        body["content"].as_str().unwrap().ends_with("(3 rolls)"),
        "{body}"
    );
    assert_eq!(
        body["components"][0]["components"][0]["custom_id"],
        "roll:3"
    );
}

#[test]
fn select_menus_read_values() {
    let (_, response) = send(
        &Bot,
        command(json!({ "id": "1", "name": "colors", "type": 1 })),
    );
    assert_eq!(custom_id(&response), "colors");
    let (_, response) = send(&Bot, click("colors", &["red", "blue"]));
    assert_eq!(response["data"]["content"], "You picked red, blue");
    assert_eq!(response["data"]["flags"], 64);
}

#[test]
fn modals_show_and_submit_typed_forms() {
    let (_, response) = send(
        &Bot,
        command(json!({ "id": "1", "name": "feedback", "type": 1 })),
    );
    assert_eq!(response["type"], 9);
    assert_eq!(response["data"]["custom_id"], "feedback");
    assert_eq!(response["data"]["title"], "Feedback");
    let inputs: Vec<_> = response["data"]["components"]
        .as_array()
        .unwrap()
        .iter()
        .map(|label| {
            assert_eq!(label["type"], 18, "every field is a Label");
            let input = &label["component"];
            (input["custom_id"].clone(), input["type"].clone())
        })
        .collect();
    assert_eq!(
        inputs,
        [
            (json!("topic"), json!(4)),
            (json!("kind"), json!(21)),
            (json!("details"), json!(4)),
            (json!("screenshots"), json!(19)),
            (json!("contact"), json!(23)),
        ]
    );
    assert_eq!(
        response["data"]["components"][1]["component"]["options"][2],
        json!({ "label": "Something nice", "value": "Praise" })
    );

    let label = |child: Value| json!({ "type": 18, "id": 1, "component": child });
    let submit = json!({
        "id": "1", "application_id": "1", "type": 5, "token": "t", "version": 1,
        "data": { "custom_id": "feedback", "components": [
            label(json!({ "type": 4, "custom_id": "topic", "value": "Docs" })),
            label(json!({ "type": 21, "custom_id": "kind", "value": "Idea" })),
            label(json!({ "type": 4, "custom_id": "details", "value": "" })),
            label(json!({ "type": 19, "custom_id": "screenshots", "values": [] })),
            label(json!({ "type": 23, "custom_id": "contact", "value": true })),
        ] },
    });
    let (_, response) = send(&Bot, submit);
    assert_eq!(
        response["data"]["content"],
        "Thanks! Idea **Docs**: (none) (0 screenshots); we'll be in touch"
    );
    assert_eq!(response["data"]["flags"], 64);
}

fn autocomplete(name: &str, option: &str, typed: &str) -> Value {
    let mut payload = command(json!({ "id": "1", "name": name, "type": 1, "options": [
        { "name": option, "type": 3, "value": typed, "focused": true },
    ] }));
    payload["type"] = json!(4);
    payload
}

#[test]
fn autocomplete_suggests_while_typing() {
    let (_, response) = send(&Bot, autocomplete("fruit", "name", "Bl"));
    assert_eq!(
        response,
        json!({ "type": 8, "data": { "choices": [{ "name": "blueberry", "value": "blueberry" }] } })
    );

    // Suggestions from REST, with the bot token
    let env = TestEnv::default();
    let (_, response, _) = send_with(&Bot, &env, autocomplete("command-info", "name", "h"));
    assert_eq!(
        response["data"]["choices"],
        json!([{ "name": "hello", "value": "hello" }])
    );
    assert_eq!(
        env.sent()[0].1,
        "/applications/1052322265397739523/commands?with_localizations=true"
    );

    let (_, response) = send(
        &Bot,
        command(
            json!({ "id": "1", "name": "command-info", "type": 1, "options": [
            { "name": "name", "type": 3, "value": "hello" },
        ] }),
        ),
    );
    assert_eq!(response["data"]["content"], "`/hello`: Say hello");
}

/// (status, Background Work spawned) for a signed Webhook Events request
fn send_event(env: &TestEnv, payload: Value, tamper: bool) -> (u16, usize) {
    let body = payload.to_string().into_bytes();
    let key = SigningKey::from_bytes(&KEY);
    let signature = hex::encode(key.sign(&[TIMESTAMP.as_bytes(), &body].concat()).to_bytes());
    let body = if tamper {
        [&body[..], b" "].concat()
    } else {
        body
    };
    let outcome = block_on(handle_event(
        &hex::encode(key.verifying_key().as_bytes()),
        Some(&signature),
        Some(TIMESTAMP),
        &body,
        Bot,
        env,
    ));
    (outcome.status(), env.work.lock().unwrap().len())
}

fn authorized(integration_type: u8) -> Value {
    json!({
        "version": 1, "application_id": "1052322265397739523", "type": 1,
        "event": { "type": "APPLICATION_AUTHORIZED", "timestamp": "2024-10-18T14:42:53.064834",
                   "data": { "integration_type": integration_type, "user": alice(),
                             "scopes": ["applications.commands"] } },
    })
}

#[test]
fn webhook_events_are_acknowledged_then_handled() {
    // Discord verifies the URL with a Ping
    let env = TestEnv::default();
    let ping = json!({ "version": 1, "application_id": "1052322265397739523", "type": 0 });
    assert_eq!(send_event(&env, ping, false), (204, 0));

    // A user install: acknowledged at once, the welcome DM is sent afterwards
    let env = TestEnv::default();
    assert_eq!(send_event(&env, authorized(1), false), (204, 1));
    assert!(env.sent().is_empty());
    env.run_background().unwrap();
    let paths: Vec<_> = env.sent().into_iter().map(|(_, path, ..)| path).collect();
    assert_eq!(paths, ["/users/@me/channels", "/channels/77/messages"]);

    // A server install is ignored
    let env = TestEnv::default();
    send_event(&env, authorized(0), false);
    env.run_background().unwrap();
    assert!(env.sent().is_empty());

    // Discord sends bad signatures on purpose; they must be refused
    assert_eq!(
        send_event(&TestEnv::default(), authorized(1), true),
        (401, 0)
    );
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
            (None, json!("fruit"), json!(1)),
            (None, json!("greet"), json!(1)),
            (Some("dev"), json!("mod"), json!(1)),
            (None, json!("Avatar"), json!(2)),
            (None, json!("Word count"), json!(3)),
            (None, json!("whoami"), json!(1)),
            (None, json!("remind"), json!(1)),
            (None, json!("countdown"), json!(1)),
            (Some("dev"), json!("announce"), json!(1)),
            (None, json!("Thumbs up"), json!(3)),
            (None, json!("dm-me"), json!(1)),
            (None, json!("report"), json!(1)),
            (None, json!("command-info"), json!(1)),
            (None, json!("export"), json!(1)),
            (None, json!("counter"), json!(1)),
            (None, json!("roll"), json!(1)),
            (None, json!("colors"), json!(1)),
            (None, json!("feedback"), json!(1)),
            (None, json!("card"), json!(1)),
            (None, json!("dice"), json!(1)),
        ]
    );
}
