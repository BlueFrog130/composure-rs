//! Every example Command, component, modal and Webhook Event on the host, through
//! `composure::testing`: a `TestPlatform` stands in for an Adapter's State (`Worker`, `Host`), with
//! Bot Data, a Transport answering the few Discord reads with canned bodies, and Background Work
//! run on demand. Plus the Definitions Sync would publish.

use composure::{
    adapter::handle_event,
    command::CommandSetDefinition,
    rest::Method,
    testing::{
        self, APPLICATION_ID, CHANNEL_ID, INTERACTION_TOKEN, MESSAGE_ID, MockTransport,
        ResponseExt, TestKey, TestPlatform, json_response, users,
    },
};
use composure_example_bot::{Bot, Config, style};
use serde_json::{Value, json};

/// Discord, answering the reads the example makes
fn discord() -> MockTransport {
    MockTransport::new().answering(|request| {
        let path = request.url.rsplit("/v10").next()?;
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
            _ => return None,
        };
        Some(json_response(200, &body))
    })
}

fn platform() -> TestPlatform<Config> {
    TestPlatform::new()
        .with_transport(discord())
        .with_data(Config {
            greeting: "Howdy".into(),
        })
}

/// Edits of the original response go here
fn original() -> String {
    format!("/webhooks/{APPLICATION_ID}/{INTERACTION_TOKEN}/messages/@original")
}

/// The response's text, which must not be an error
fn text(interaction: impl Into<composure::types::Interaction>) -> String {
    let response = testing::run(&Bot, interaction, &platform()).unwrap();
    response.text().unwrap_or_default()
}

#[test]
fn chat_input_commands() {
    assert_eq!(text(testing::command("hello")), "Hello, Alice!");
    assert_eq!(
        text(
            testing::command("echo")
                .option("text", "hi")
                .option("times", 3)
        ),
        "hi hi hi"
    );

    let response = testing::answer(
        &Bot,
        testing::command("ice-cream").option("flavor", "mint"),
        &platform(),
    );
    let embed = &response.embeds()[0];
    assert_eq!(embed["title"], "1 x Mint chip");
    assert_eq!(embed["color"], style::PRIMARY);
    assert_eq!(embed["footer"]["text"], "composure example");

    assert_eq!(text(testing::command("greet")), "Howdy from Composure!");
}

#[test]
fn group_and_context_menu_commands() {
    let ban = testing::command("mod")
        .subcommand("ban")
        .user_option("user", &users::alice());
    let response = testing::answer(&Bot, ban, &platform());
    let error = &response.embeds()[0];
    assert_eq!(error["description"], "You can't ban yourself.");
    assert_eq!(error["color"], style::ERROR);
    assert_eq!(error["footer"]["text"], "composure example");
    assert!(response.is_ephemeral());

    let ban = testing::command("mod")
        .subcommand("ban")
        .user_option("user", &users::bob())
        .option("reason", "spam");
    let response = testing::answer(&Bot, ban, &platform());
    let embed = &response.embeds()[0];
    assert_eq!(embed["description"], "Would ban Bob (spam)");
    assert_eq!(embed["color"], style::WARNING);
    assert!(response.is_ephemeral());

    let mut alice = users::alice();
    alice.avatar = Some("abc".into());
    assert_eq!(
        text(testing::user_command("Avatar", &alice)),
        format!("https://cdn.discordapp.com/avatars/{}/abc.png", alice.id)
    );
}

#[test]
fn inline_rest_with_the_bot_token() {
    let env = platform();
    let response = testing::answer(&Bot, testing::command("whoami"), &env);
    assert_eq!(response.text().as_deref(), Some("I'm Composure."));
    assert_eq!(
        env.transport().request(0).header("authorization"),
        Some("Bot bot-token")
    );

    let env = platform();
    let response = testing::answer(
        &Bot,
        testing::command("announce").option("text", "Release day"),
        &env,
    );
    assert_eq!(response.text().as_deref(), Some("Posted."));
    assert_eq!(
        env.transport().sent(),
        [(Method::Post, format!("/channels/{CHANNEL_ID}/messages"))]
    );
    let request = env.transport().request(0);
    assert_eq!(request.header("content-type"), Some("application/json"));
    assert_eq!(env.transport().body(0), json!({ "content": "Release day" }));

    let env = platform();
    let response = testing::answer(&Bot, testing::command("dm-me").option("text", "psst"), &env);
    assert_eq!(response.text().as_deref(), Some("Check your DMs."));
    assert_eq!(
        env.transport().paths(),
        ["/users/@me/channels", "/channels/77/messages"]
    );
}

#[test]
fn message_command_reacts() {
    let env = platform();
    let response = testing::answer(&Bot, testing::message_command("Thumbs up", "nice"), &env);
    assert_eq!(response.text().as_deref(), Some("Reacted."));
    assert_eq!(
        env.transport().paths()[0],
        format!("/channels/{CHANNEL_ID}/messages/{MESSAGE_ID}/reactions/%F0%9F%91%8D/@me")
    );
}

#[test]
fn deferred_commands_finish_in_the_background() {
    let env = platform();
    let response = testing::answer(
        &Bot,
        testing::command("remind")
            .option("text", "stretch")
            .option("seconds", 2),
        &env,
    );
    // Answered right away; nothing sent to Discord until the response is out
    assert_eq!(
        response.json(),
        json!({ "type": 5, "data": { "flags": 64 } })
    );
    assert!(env.transport().requests().is_empty());

    env.run_background().unwrap();
    let sent = env.transport().sent();
    assert_eq!(sent.len(), 2);
    assert_eq!(
        sent[0],
        (
            Method::Post,
            format!("/webhooks/{APPLICATION_ID}/{INTERACTION_TOKEN}")
        )
    );
    let alice = users::alice().id;
    assert!(
        env.transport()
            .text(0)
            .contains(&format!("<@{alice}> stretch"))
    );
    assert_eq!(sent[1], (Method::Patch, original()));
    assert_eq!(
        env.transport().body(1),
        json!({ "content": "Reminded you about \"stretch\"." })
    );

    let env = platform();
    let response = testing::answer(&Bot, testing::command("countdown"), &env);
    assert_eq!(response.json(), json!({ "type": 5 }));
    env.run_background().unwrap();
    let edits: Vec<_> = (0..4)
        .map(|i| env.transport().body(i)["content"].clone())
        .collect();
    assert_eq!(edits, ["3...", "2...", "1...", "Liftoff! \u{1F680}"]);
}

#[test]
fn files_upload_as_multipart() {
    // In the initial response: the Adapter sends it through the callback endpoint
    let response = testing::answer(
        &Bot,
        testing::command("export").option("text", "hello file"),
        &platform(),
    );
    assert_eq!(response.text().as_deref(), Some("Here's your file:"));
    assert_eq!(response.files().len(), 1);

    // In a deferred result: with the edit of the original response
    let env = platform();
    testing::answer(&Bot, testing::command("report"), &env);
    env.run_background().unwrap();
    assert_eq!(
        env.transport().paths()[0],
        format!("/applications/{APPLICATION_ID}/commands?with_localizations=true")
    );
    let edit = env.transport().request(1);
    assert_eq!(env.transport().sent()[1], (Method::Patch, original()));
    assert!(
        edit.header("content-type")
            .unwrap()
            .starts_with("multipart/form-data")
    );
    let body = env.transport().text(1);
    assert!(body.contains("name,type\nhello,slash\nAvatar,user\n"));
    assert!(body.contains(r#""content":"2 global commands""#));
}

#[test]
fn errors_become_ephemeral_styled_embeds() {
    let response = testing::answer(&Bot, testing::command("nope"), &platform());
    let error = &response.embeds()[0];
    assert_eq!(error["description"], "Unknown command.");
    assert_eq!(error["color"], style::ERROR);
    assert!(response.is_ephemeral());

    // `text` is required
    let response = testing::answer(&Bot, testing::command("echo"), &platform());
    assert!(response.is_ephemeral());
}

/// The first component's `custom_id`
fn first_custom_id(response: &impl ResponseExt) -> String {
    response.components()[1]["custom_id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn buttons_carry_state_in_the_custom_id() {
    let response = testing::answer(&Bot, testing::command("counter"), &platform());
    assert_eq!(response.text().as_deref(), Some("Clicked 0 times"));
    assert_eq!(first_custom_id(&response), "counter:0");

    let response = testing::answer(&Bot, testing::component("counter:4"), &platform());
    assert_eq!(response.response_type(), 7, "updates the clicked message");
    assert_eq!(response.text().as_deref(), Some("Clicked 5 times"));
    assert_eq!(first_custom_id(&response), "counter:5");

    // No handler for the prefix: the default "not handled" message
    let response = testing::answer(&Bot, testing::component("button"), &platform());
    assert_eq!(
        response.text().as_deref(),
        Some("This bot does not handle components.")
    );
}

#[test]
fn components_v2_card_keeps_its_accent_and_updates_in_place() {
    let response = testing::answer(&Bot, testing::command("card"), &platform());
    let data = &response.json()["data"];
    assert_eq!(data["flags"], 1 << 15, "IS_COMPONENTS_V2");
    assert_eq!(data.get("content"), None);
    let container = &data["components"][0];
    assert_eq!(container["type"], 17);
    assert_eq!(container["accent_color"], style::PRIMARY);
    assert_eq!(container["components"][0]["accessory"]["style"], 5);
    assert_eq!(container["components"][2]["content"], "-# 0 likes");
    assert_eq!(
        container["components"][3]["components"][0]["custom_id"],
        "card:like:0"
    );

    let response = testing::answer(&Bot, testing::component("card:like:2"), &platform());
    assert_eq!(response.response_type(), 7);
    let container = &response.json()["data"]["components"][0];
    assert_eq!(container["components"][2]["content"], "-# 3 likes");
    assert_eq!(container["accent_color"], style::PRIMARY);
    assert_eq!(
        container["components"][3]["components"][1]["custom_id"],
        "card:review:3"
    );
}

#[test]
fn view_opens_a_modal_view_and_reads_its_fields() {
    let response = testing::answer(&Bot, testing::component("card:review:3"), &platform());
    assert_eq!(response.response_type(), 9);
    let data = &response.json()["data"];
    assert_eq!(data["custom_id"], "review:send:3");
    assert_eq!(
        data["components"][0]["component"]["options"][1]["label"],
        "★★"
    );
    assert_eq!(data["components"][1]["component"]["required"], false);

    let submit = testing::modal("review:send:3")
        .input(json!({ "type": 21, "custom_id": "stars", "value": "Three" }))
        .field("comment", "neat");
    let response = testing::answer(&Bot, submit, &platform());
    assert_eq!(
        response.text().as_deref(),
        Some("★★★ for a card with 3 likes: neat")
    );
    assert!(response.is_ephemeral());
}

#[test]
fn view_handler_defers_and_rerenders_later() {
    let env = platform();
    let response = testing::answer(&Bot, testing::component("dice:roll:2"), &env);
    assert_eq!(response.json(), json!({ "type": 6 }));

    env.run_background().unwrap();
    assert_eq!(env.transport().sent()[0], (Method::Patch, original()));
    let body = env.transport().body(0);
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
    let env = platform();
    let response = testing::answer(&Bot, testing::component("roll:2"), &env);
    assert_eq!(response.json(), json!({ "type": 6 }));
    assert!(env.transport().requests().is_empty());

    env.run_background().unwrap();
    assert_eq!(env.transport().sent()[0], (Method::Patch, original()));
    let body = env.transport().body(0);
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
    let response = testing::answer(&Bot, testing::command("colors"), &platform());
    assert_eq!(first_custom_id(&response), "colors");
    let response = testing::answer(
        &Bot,
        testing::component("colors").values(["red", "blue"]),
        &platform(),
    );
    assert_eq!(response.text().as_deref(), Some("You picked red, blue"));
    assert!(response.is_ephemeral());
}

#[test]
fn modals_show_and_submit_typed_forms() {
    let response = testing::answer(&Bot, testing::command("feedback"), &platform());
    assert_eq!(response.response_type(), 9);
    let data = &response.json()["data"];
    assert_eq!(data["custom_id"], "feedback");
    assert_eq!(data["title"], "Feedback");
    let inputs: Vec<_> = data["components"]
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
        data["components"][1]["component"]["options"][2],
        json!({ "label": "Something nice", "value": "Praise" })
    );

    let submit = testing::modal("feedback")
        .field("topic", "Docs")
        .input(json!({ "type": 21, "custom_id": "kind", "value": "Idea" }))
        .field("details", "")
        .input(json!({ "type": 19, "custom_id": "screenshots", "values": [] }))
        .input(json!({ "type": 23, "custom_id": "contact", "value": true }));
    let response = testing::answer(&Bot, submit, &platform());
    assert_eq!(
        response.text().as_deref(),
        Some("Thanks! Idea **Docs**: (none) (0 screenshots); we'll be in touch")
    );
    assert!(response.is_ephemeral());
}

#[test]
fn autocomplete_suggests_while_typing() {
    let response = testing::answer(
        &Bot,
        testing::autocomplete("fruit").focused("name", "Bl"),
        &platform(),
    );
    assert_eq!(
        response.json(),
        json!({ "type": 8, "data": { "choices": [{ "name": "blueberry", "value": "blueberry" }] } })
    );

    // Suggestions from REST, with the bot token
    let env = platform();
    let response = testing::answer(
        &Bot,
        testing::autocomplete("command-info").focused("name", "h"),
        &env,
    );
    assert_eq!(
        response.choices(),
        [json!({ "name": "hello", "value": "hello" })]
    );
    assert_eq!(
        env.transport().paths()[0],
        format!("/applications/{APPLICATION_ID}/commands?with_localizations=true")
    );

    assert_eq!(
        text(testing::command("command-info").option("name", "hello")),
        "`/hello`: Say hello"
    );
}

/// The status of a signed Webhook Events request, `tampered` with after signing or not
fn send_event(env: &TestPlatform<Config>, payload: Value, tampered: bool) -> u16 {
    let key = TestKey::default();
    let mut request = key.signed(payload.to_string());
    if tampered {
        request.body.push(b' ');
    }
    let outcome = testing::block_on(handle_event(
        &key.public_key(),
        Some(&request.signature),
        Some(&request.timestamp),
        &request.body,
        Bot,
        env,
    ));
    outcome.status()
}

fn authorized(integration_type: u8) -> Value {
    json!({
        "version": 1, "application_id": APPLICATION_ID, "type": 1,
        "event": { "type": "APPLICATION_AUTHORIZED", "timestamp": "2024-10-18T14:42:53.064834",
                   "data": { "integration_type": integration_type,
                             "user": { "id": users::alice().id, "username": "alice", "discriminator": "0" },
                             "scopes": ["applications.commands"] } },
    })
}

#[test]
fn webhook_events_are_acknowledged_then_handled() {
    // Discord verifies the URL with a Ping
    let env = platform();
    let ping = json!({ "version": 1, "application_id": APPLICATION_ID, "type": 0 });
    assert_eq!(send_event(&env, ping, false), 204);
    assert_eq!(env.queued_work(), 0);

    // A user install: acknowledged at once, the welcome DM is sent afterwards
    let env = platform();
    assert_eq!(send_event(&env, authorized(1), false), 204);
    assert_eq!(env.queued_work(), 1);
    assert!(env.transport().requests().is_empty());
    env.run_background().unwrap();
    assert_eq!(
        env.transport().paths(),
        ["/users/@me/channels", "/channels/77/messages"]
    );

    // A server install is ignored
    let env = platform();
    send_event(&env, authorized(0), false);
    env.run_background().unwrap();
    assert!(env.transport().requests().is_empty());

    // Discord sends bad signatures on purpose; they must be refused
    let env = platform();
    assert_eq!(send_event(&env, authorized(1), true), 401);
    assert_eq!(env.queued_work(), 0);
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
