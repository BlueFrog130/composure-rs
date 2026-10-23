//! Interactions, events and the cron work through the bot on the host, with
//! `composure::testing`. `TestEnv` stands in for `Worker<Config>`: a `TestPlatform` whose
//! Transport answers Discord, GitHub and crates.io with canned bodies, plus a map for KV.

use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use composure::{
    adapter::handle_event,
    command::{BackgroundWork, CommandSetDefinition, HandlerError, HasData, Platform},
    rest::{BotClient, Method, RawResponse, Request},
    testing::{
        self, APPLICATION_ID, CHANNEL_ID, GUILD_ID, INTERACTION_TOKEN, InteractionBuilder,
        MESSAGE_ID, MockTransport, ResponseExt, TestKey, TestPlatform, json_response, users,
    },
    types::{Interaction, Permissions},
};
use composure_bot::{ComposureBot, Config, Store, notify};
use serde_json::{Value, json};

const NEWS: &str = "300000000000000002";
const ANNOUNCE: &str = "600000000000000001";
const FEEDBACK: &str = "600000000000000002";
const LOG: &str = "600000000000000003";

fn config() -> Config {
    let vars = HashMap::from([
        ("ANNOUNCE_CHANNEL_ID", ANNOUNCE),
        ("FEEDBACK_CHANNEL_ID", FEEDBACK),
        ("LOG_CHANNEL_ID", LOG),
        ("GITHUB_TOKEN", "gh-token"),
    ]);
    Config::from_lookup(|name| vars.get(name).map(|value| value.to_string())).unwrap()
}

fn github_releases() -> Value {
    let release = |tag: &str, at: &str, body: &str, draft: bool, prerelease: bool| {
        json!({
            "tag_name": tag, "name": tag, "draft": draft, "prerelease": prerelease,
            "published_at": at, "body": body,
            "html_url": format!("https://github.com/BlueFrog130/composure-rs/releases/tag/{tag}"),
        })
    };
    json!([
        release(
            "v0.2.0-rc.1",
            "2026-10-21T10:00:00Z",
            "## Features\n\nSoon",
            false,
            true
        ),
        release("v0.3.0", "2026-10-22T10:00:00Z", "draft", true, false),
        release(
            "v0.1.2",
            "2026-10-20T17:05:02Z",
            "## Features\n\n### Faster cold starts\n\nWorkers compile\neach function lazily.",
            false,
            false
        ),
        release(
            "v0.1.1",
            "2026-10-20T15:09:25Z",
            "## Fixes\n\n### Deferred update errors",
            false,
            false
        ),
        release(
            "v0.1.0",
            "2026-10-20T14:20:53Z",
            "## Features\n\n### Commands defined once",
            false,
            false
        ),
    ])
}

fn message(id: &str, channel: &str) -> Value {
    json!({
        "id": id, "channel_id": channel,
        "author": { "id": users::alice().id, "username": "alice", "discriminator": "0" },
        "content": "", "timestamp": "2026-10-20T00:00:00.000000+00:00", "edited_timestamp": null,
        "tts": false, "mention_everyone": false, "mentions": [], "mention_roles": [],
        "attachments": [], "embeds": [], "pinned": false, "type": 0,
    })
}

fn follower(id: &str, channel: &str) -> Value {
    json!({
        "id": id, "type": 2, "guild_id": GUILD_ID, "channel_id": channel, "name": "Composure #news",
        "avatar": null, "application_id": null,
        "source_guild": { "id": "1", "name": "Composure", "icon": null },
        "source_channel": { "id": ANNOUNCE, "name": "releases" },
    })
}

/// What Discord, GitHub and crates.io hold for the test
#[derive(Default)]
struct World {
    /// This guild's webhooks
    webhooks: Mutex<Vec<Value>>,
    /// Answer webhook requests with 403
    forbidden: AtomicBool,
}

impl World {
    fn answer(&self, request: &Request) -> RawResponse {
        if request.url.starts_with("https://api.github.com/") {
            return json_response(200, &github_releases());
        }
        if let Some(name) = request.url.strip_prefix("https://crates.io/api/v1/crates/") {
            return json_response(
                200,
                &json!({ "crate": {
                    "name": name, "description": "HTTP-only Discord bots", "max_version": "0.2.0-rc.1",
                    "max_stable_version": "0.1.2", "downloads": 12345, "recent_downloads": 678,
                    "updated_at": "2026-10-20T17:05:02.000000+00:00",
                } }),
            );
        }
        let path = request.url.rsplit("/v10").next().unwrap();
        let webhooks = path.contains("/webhooks") && !path.starts_with("/webhooks/1052");
        if webhooks && self.forbidden.load(Ordering::Relaxed) {
            return json_response(
                403,
                &json!({ "message": "Missing Permissions", "code": 50013 }),
            );
        }
        match (request.method, path) {
            (Method::Get, p) if p == format!("/guilds/{GUILD_ID}/webhooks") => {
                json_response(200, &Value::Array(self.webhooks.lock().unwrap().clone()))
            }
            (Method::Post, p) if p.ends_with("/followers") => json_response(
                200,
                &json!({ "channel_id": ANNOUNCE, "webhook_id": "800000000000000009" }),
            ),
            (Method::Delete, _) => RawResponse {
                status: 204,
                headers: Vec::new(),
                body: Vec::new(),
            },
            (Method::Post, "/users/@me/channels") => {
                json_response(200, &json!({ "id": "77", "type": 1 }))
            }
            (Method::Post, p) if p.starts_with("/channels/") => {
                let channel = p.split('/').nth(2).unwrap();
                json_response(200, &message("700000000000000001", channel))
            }
            _ => json_response(200, &json!({})),
        }
    }
}

/// The bot's State: a `TestPlatform` (Transport, Bot Data, Background Work) plus the `Store`
/// the bot's handlers need, which an integration test can't implement on `TestPlatform` itself
#[derive(Clone)]
struct TestEnv {
    platform: TestPlatform<Config>,
    world: Arc<World>,
    kv: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Default for TestEnv {
    fn default() -> Self {
        let world = Arc::new(World::default());
        let answering = world.clone();
        let transport =
            MockTransport::new().answering(move |request| Some(answering.answer(request)));
        Self {
            platform: TestPlatform::new()
                .with_transport(transport)
                .with_data(config()),
            world,
            kv: Arc::default(),
        }
    }
}

impl HasData for TestEnv {
    type Data = Config;

    fn data(&self) -> &Config {
        self.platform.data()
    }
}

impl Platform for TestEnv {
    type Transport = <TestPlatform<Config> as Platform>::Transport;

    fn transport(&self) -> Self::Transport {
        Platform::transport(&self.platform)
    }

    fn spawn(&self, work: BackgroundWork) {
        self.platform.spawn(work);
    }

    fn bot_token(&self) -> Option<&str> {
        self.platform.bot_token()
    }
}

impl Store for TestEnv {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, HandlerError> {
        Ok(self.kv.lock().unwrap().get(key).cloned())
    }

    async fn put(
        &self,
        key: &str,
        value: Vec<u8>,
        _ttl: Option<Duration>,
    ) -> Result<(), HandlerError> {
        self.kv.lock().unwrap().insert(key.into(), value);
        Ok(())
    }
}

impl TestEnv {
    fn run_background(&self) -> Result<(), HandlerError> {
        self.platform.run_background()
    }

    /// (method, URL without Discord's base, body) of every request sent
    fn sent(&self) -> Vec<(Method, String, String)> {
        let transport = self.platform.transport();
        transport
            .sent()
            .into_iter()
            .enumerate()
            .map(|(i, (method, path))| (method, path, transport.text(i)))
            .collect()
    }

    fn paths(&self) -> Vec<String> {
        self.platform.transport().paths()
    }

    fn github_calls(&self) -> usize {
        self.paths()
            .iter()
            .filter(|path| path.starts_with("https://api.github.com/"))
            .count()
    }

    /// The response to `interaction`, errors answered as the Adapter answers them
    fn answer(&self, interaction: impl Into<Interaction>) -> Value {
        testing::answer(&ComposureBot, interaction, self).json()
    }
}

/// In the guild, by a member allowed to manage it
fn in_guild<D>(interaction: InteractionBuilder<D>) -> InteractionBuilder<D> {
    interaction
        .in_guild(GUILD_ID)
        .permissions(Permissions::ManageGuild)
}

fn command(name: &str) -> InteractionBuilder<testing::CommandData> {
    in_guild(testing::command(name))
}

fn autocomplete(name: &str, option: &str, typed: &str) -> InteractionBuilder<testing::CommandData> {
    in_guild(testing::autocomplete(name).focused(option, typed))
}

fn click(custom_id: &str) -> InteractionBuilder<testing::ComponentData> {
    in_guild(testing::component(custom_id))
}

/// The text displays' content, joined
fn text(response: &Value) -> String {
    response.text().unwrap_or_default()
}

/// The component labeled `label`
fn button(response: &Value, label: &str) -> Value {
    response
        .button(label)
        .unwrap_or_else(|| panic!("no {label} in {response}"))
}

fn custom_id(response: &Value, label: &str) -> String {
    response.custom_id(label).unwrap()
}

/// Edits of the original response go here
fn original() -> String {
    format!("/webhooks/{APPLICATION_ID}/{INTERACTION_TOKEN}/messages/@original")
}

#[test]
fn syncs_every_command_with_its_contexts() {
    let definitions: Vec<Value> = ComposureBot::definitions()
        .iter()
        .map(|definition| serde_json::to_value(&definition.command).unwrap())
        .collect();
    let names: Vec<_> = definitions
        .iter()
        .map(|d| d["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "changelog",
            "compare",
            "Explain release",
            "notify",
            "docs",
            "crate",
            "feedback",
            "about",
            "invite"
        ]
    );
    for definition in &definitions {
        if definition["name"] == "notify" {
            assert_eq!(definition["contexts"], json!([0]));
            assert_eq!(definition["integration_types"], json!([0]));
            assert_eq!(definition["default_member_permissions"], "32");
        } else {
            assert_eq!(definition["contexts"], json!([0, 1, 2]), "{definition}");
            assert_eq!(
                definition["integration_types"],
                json!([0, 1]),
                "{definition}"
            );
        }
    }
}

#[test]
fn changelog_opens_on_the_latest_release_and_pages() {
    let env = TestEnv::default();
    let alice = users::alice().id;
    let response = env.answer(command("changelog"));
    assert_eq!(response["type"], 4);
    let page = text(&response);
    assert!(page.contains("## composure v0.1.2"), "{page}");
    assert!(page.contains("Released 2026-10-20 · 1 of 3"), "{page}");
    // Headings moved down a level, hard wraps joined
    assert!(
        page.contains(
            "### Features\n\n**Faster cold starts**\n\nWorkers compile each function lazily."
        ),
        "{page}"
    );
    assert_eq!(button(&response, "◀ Newer")["disabled"], true);
    assert_eq!(
        button(&response, "On GitHub")["url"],
        "https://github.com/BlueFrog130/composure-rs/releases/tag/v0.1.2"
    );
    // The jump menu lists published releases only
    let select = response
        .components()
        .into_iter()
        .find(|c| c["type"] == 3)
        .unwrap();
    let options: Vec<_> = select["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["value"].clone())
        .collect();
    assert_eq!(options, [json!("0.1.2"), json!("0.1.1"), json!("0.1.0")]);
    assert_eq!(select["options"][0]["default"], true);

    // The owner turns the page in place
    let older = custom_id(&response, "Older ▶");
    assert_eq!(older, format!("cl:older:{alice}:::0.1.1"));
    let turned = env.answer(click(&older));
    assert_eq!(turned["type"], 7);
    assert!(text(&turned).contains("## composure v0.1.1"));
    assert!(text(&turned).contains("2 of 3"));

    // Anyone else gets a copy of their own
    let bob = users::bob();
    let copy = env.answer(click(&older).by(&bob));
    assert_eq!(copy["type"], 4);
    assert!(copy.is_ephemeral());
    assert!(custom_id(&copy, "Older ▶").starts_with(&format!("cl:older:{}:", bob.id)));

    // The jump menu
    let select = turned
        .components()
        .into_iter()
        .find(|c| c["type"] == 3)
        .unwrap();
    let jump = select["custom_id"].as_str().unwrap();
    let jumped = env.answer(click(jump).values(["0.1.0"]));
    assert!(text(&jumped).contains("## composure v0.1.0"));
    assert_eq!(button(&jumped, "Older ▶")["disabled"], true);

    // GitHub was asked once; every page after came from the store
    assert_eq!(env.github_calls(), 1);
    let request = env.platform.transport().request(0);
    assert_eq!(request.header("authorization"), Some("Bearer gh-token"));
    assert!(
        request
            .header("user-agent")
            .unwrap()
            .starts_with("composure-bot")
    );
}

#[test]
fn changelog_versions_autocomplete_and_errors() {
    let env = TestEnv::default();
    let response = env.answer(autocomplete("changelog", "version", "v0.1"));
    assert_eq!(
        response.choices(),
        [
            json!({ "name": "v0.1.2 (2026-10-20)", "value": "0.1.2" }),
            json!({ "name": "v0.1.1 (2026-10-20)", "value": "0.1.1" }),
            json!({ "name": "v0.1.0 (2026-10-20)", "value": "0.1.0" }),
        ]
    );

    let response = env.answer(command("changelog").option("version", "v0.1.1"));
    assert!(text(&response).contains("## composure v0.1.1"));

    let response = env.answer(command("changelog").option("version", "9.9.9"));
    assert!(response.is_ephemeral());
    assert!(
        response.to_string().contains("There is no release `9.9.9`"),
        "{response}"
    );
    // Rendered by the bot's `errors = ui::render_error`: its error container, ephemeral
    assert_eq!(
        response.components()[0]["accent_color"],
        composure_bot::ui::ERROR
    );
    assert_eq!(response["data"]["flags"], 64 | (1 << 15));
}

#[test]
fn compare_pages_through_a_range() {
    let env = TestEnv::default();
    let response = env.answer(command("compare").option("from", "0.1.1"));
    let page = text(&response);
    assert!(
        page.contains("Changes from v0.1.1 to v0.1.2: 2 releases"),
        "{page}"
    );
    assert!(page.contains("1 of 2"));
    let older = custom_id(&response, "Older ▶");
    assert_eq!(
        older,
        format!("cl:older:{}:0.1.1:0.1.2:0.1.1", users::alice().id)
    );
    let last = env.answer(click(&older));
    assert!(text(&last).contains("2 of 2"));
    assert_eq!(button(&last, "Older ▶")["disabled"], true);
}

#[test]
fn explain_release_reads_the_version_from_a_message() {
    let env = TestEnv::default();
    let explain = in_guild(testing::message_command("Explain release", "")).json(|json| {
        json["data"]["resolved"]["messages"][MESSAGE_ID.to_string()]["embeds"] =
            json!([{ "title": "composure v0.1.1 is out" }]);
    });
    let response = env.answer(explain);
    assert!(response.is_ephemeral());
    assert!(text(&response).contains("## composure v0.1.1"));

    // An announcement is Components V2: its text is in Text Displays
    let announcement = |content: &str| {
        let content = content.to_string();
        in_guild(testing::message_command("Explain release", "")).json(move |json| {
            json["data"]["resolved"]["messages"][MESSAGE_ID.to_string()]["components"] = json!([
                { "type": 17, "components": [{ "type": 10, "content": content }] }
            ]);
        })
    };
    let response = env.answer(announcement("## composure v0.1.0 is out"));
    assert!(
        text(&response).contains("## composure v0.1.0"),
        "{response}"
    );

    // composurejs's bot announces its own versions in the same server
    let response = env.answer(announcement("## composurejs v0.1.2 is out"));
    assert!(
        response
            .to_string()
            .contains("doesn't mention a composure version"),
        "{response}"
    );
}

#[test]
fn notify_follows_lists_and_removes() {
    let env = TestEnv::default();
    // Channel Options arrive with the channel in `resolved`
    let notify = |subcommand: &str| {
        command("notify")
            .subcommand(subcommand)
            .resolved(json!({ "channels": { NEWS: { "id": NEWS, "type": 0, "name": "news", "permissions": "0" } } }))
    };
    let setup =
        || notify("setup").option_json(json!({ "name": "channel", "type": 7, "value": NEWS }));

    let response = env.answer(setup());
    assert!(
        response
            .to_string()
            .contains(&format!("announced in <#{NEWS}>")),
        "{response}"
    );
    let sent = env.sent();
    assert_eq!(sent[0].1, format!("/guilds/{GUILD_ID}/webhooks"));
    assert_eq!(sent[1].0, Method::Post);
    assert_eq!(sent[1].1, format!("/channels/{ANNOUNCE}/followers"));
    assert_eq!(sent[1].2, format!(r#"{{"webhook_channel_id":"{NEWS}"}}"#));

    // Following is found again through the guild's webhooks
    env.world
        .webhooks
        .lock()
        .unwrap()
        .push(follower("800000000000000009", NEWS));
    env.world.webhooks.lock().unwrap().push(json!({
        "id": "800000000000000010", "type": 1, "guild_id": GUILD_ID, "channel_id": CHANNEL_ID,
        "name": "Someone else's", "avatar": null, "application_id": null,
    }));
    let response = env.answer(setup());
    assert!(
        response
            .to_string()
            .contains("already gets release announcements")
    );
    let response = env.answer(notify("status"));
    assert!(
        response.to_string().contains(&format!("go to <#{NEWS}>.")),
        "{response}"
    );

    let before = env.sent().len();
    let response = env.answer(notify("remove"));
    assert!(
        response
            .to_string()
            .contains(&format!("Stopped release announcements in <#{NEWS}>."))
    );
    let sent = env.sent();
    assert_eq!(sent[before + 1].0, Method::Delete);
    assert_eq!(sent[before + 1].1, "/webhooks/800000000000000009");
    assert_eq!(
        sent.len(),
        before + 2,
        "only the follower webhook is deleted"
    );
}

#[test]
fn notify_explains_missing_permissions() {
    let env = TestEnv::default();
    env.world.forbidden.store(true, Ordering::Relaxed);
    let response = env.answer(command("notify").subcommand("status"));
    assert!(response.is_ephemeral());
    assert!(
        response.to_string().contains("Manage Webhooks"),
        "{response}"
    );
}

#[test]
fn notify_preview_shows_the_announcement() {
    let env = TestEnv::default();
    let response = env.answer(command("notify").subcommand("preview"));
    // Components V2, ephemeral
    assert_eq!(response["data"]["flags"], 64 | (1 << 15));
    let container = &response["data"]["components"][0];
    assert_eq!(container["accent_color"], 0xd34516);
    let page = text(&response);
    assert!(page.contains("## composure v0.1.2 is out"), "{page}");
    assert!(page.contains("Released 2026-10-20"), "{page}");
    assert!(page.contains(composure_bot::ui::FOOTER), "{page}");
    let links: Vec<_> = response
        .components()
        .into_iter()
        .filter(|c| c["type"] == 2)
        .map(|button| button["url"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        links,
        [
            "https://github.com/BlueFrog130/composure-rs/releases/tag/v0.1.2",
            "https://crates.io/crates/composure/0.1.2",
            "https://docs.rs/composure/0.1.2",
        ]
    );
}

#[test]
fn cron_announces_new_releases_oldest_first() {
    let env = TestEnv::default();
    let transport = env.transport();
    let bot = BotClient::new(transport.clone(), "bot-token", APPLICATION_ID);
    let announce =
        || testing::block_on(notify::announce_new(&bot, &transport, &env, env.data())).unwrap();

    // First run: every release, oldest first
    assert_eq!(announce(), ["0.1.0", "0.1.1", "0.1.2"]);
    assert_eq!(env.kv.lock().unwrap()["announced"], b"0.1.2");
    let posts = |env: &TestEnv| {
        env.paths()
            .iter()
            .filter(|path| path.ends_with("/messages"))
            .count()
    };
    assert_eq!(posts(&env), 3);
    assert!(announce().is_empty(), "up to date");

    // The last one announced was deleted: catch up without reposting
    env.kv
        .lock()
        .unwrap()
        .insert("announced".into(), b"0.0.9".to_vec());
    assert!(announce().is_empty());
    assert_eq!(env.kv.lock().unwrap()["announced"], b"0.1.2");
    assert_eq!(posts(&env), 3);
    transport.clear();

    // Two releases behind: both posted, oldest first, each published to followers
    env.kv
        .lock()
        .unwrap()
        .insert("announced".into(), b"0.1.0".to_vec());
    assert_eq!(announce(), ["0.1.1", "0.1.2"]);
    let discord: Vec<_> = env
        .sent()
        .into_iter()
        .filter(|(_, path, _)| path.starts_with('/'))
        .collect();
    let paths: Vec<_> = discord.iter().map(|(_, path, _)| path.as_str()).collect();
    assert_eq!(
        paths,
        [
            format!("/channels/{ANNOUNCE}/messages"),
            format!("/channels/{ANNOUNCE}/messages/700000000000000001/crosspost"),
            format!("/channels/{ANNOUNCE}/messages"),
            format!("/channels/{ANNOUNCE}/messages/700000000000000001/crosspost"),
        ]
    );
    let first: Value = serde_json::from_str(&discord[0].2).unwrap();
    assert_eq!(first["flags"], 1 << 15, "Components V2");
    assert_eq!(first["components"][0]["accent_color"], 0xd34516);
    assert!(discord[0].2.contains("composure v0.1.1 is out"));
    assert!(discord[2].2.contains("composure v0.1.2 is out"));
    assert_eq!(env.kv.lock().unwrap()["announced"], b"0.1.2");

    // Up to date
    assert!(announce().is_empty());
}

#[test]
fn docs_finds_guide_sections() {
    let env = TestEnv::default();
    let response = env.answer(autocomplete("docs", "topic", "inline"));
    let first = &response.choices()[0];
    assert_eq!(first["name"], "Views › Inline handlers");

    let picked = env.answer(command("docs").option("topic", first["value"].as_str().unwrap()));
    assert!(text(&picked).contains("### Views › Inline handlers"));
    assert_eq!(
        button(&picked, "Open")["url"],
        "https://bluefrog130.github.io/composure-rs/views.html#inline-handlers"
    );

    // Typed text without picking a suggestion is searched
    let typed = env.answer(command("docs").option("topic", "rsx"));
    assert!(text(&typed).contains("### rsx!"), "{typed}");
}

#[test]
fn crate_stats_are_deferred_and_cached() {
    let env = TestEnv::default();
    let response = env.answer(command("crate"));
    assert_eq!(response, json!({ "type": 5 }));
    env.run_background().unwrap();
    let (method, path, body) = env.sent().pop().unwrap();
    assert_eq!((method, path), (Method::Patch, original()));
    assert!(body.contains("## composure 0.1.2"), "{body}");
    assert!(
        body.contains("**12,345** downloads · **678** in the last 90 days"),
        "{body}"
    );

    env.answer(command("crate"));
    env.run_background().unwrap();
    let crates_io = env
        .paths()
        .iter()
        .filter(|p| p.starts_with("https://crates.io/"))
        .count();
    assert_eq!(crates_io, 1);
}

#[test]
fn feedback_reaches_the_maintainers() {
    let env = TestEnv::default();
    let response = env.answer(command("feedback"));
    assert_eq!(response["type"], 9);
    assert_eq!(response["data"]["custom_id"], "fb:send");

    let submit = in_guild(testing::modal("fb:send"))
        .input(json!({ "type": 21, "custom_id": "kind", "value": "Bug" }))
        .field("title", "Crash on start")
        .field("details", "It panics.");
    let response = env.answer(submit);
    assert!(response.is_ephemeral());
    assert_eq!(
        button(&response, "Open as GitHub issue")["url"],
        "https://github.com/BlueFrog130/composure-rs/issues/new?title=%5BBug%5D%20Crash%20on%20start&body=It%20panics."
    );
    let (method, path, body) = env.sent().pop().unwrap();
    assert_eq!(
        (method, path),
        (Method::Post, format!("/channels/{FEEDBACK}/messages"))
    );
    assert!(
        body.contains("### [Bug] Crash on start")
            && body.contains(&format!("<@{}>", users::alice().id)),
        "{body}"
    );
    // A Components V2 message in the error tone: bugs stand out
    let report: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(report["flags"], 1 << 15);
    assert_eq!(
        report["components"][0]["accent_color"],
        composure_bot::ui::ERROR
    );
}

#[test]
fn about_and_invite() {
    let env = TestEnv::default();
    let about = env.answer(command("about"));
    let running = format!(
        "Running composure {}",
        composure_bot::build_info::COMPOSURE_VERSION
    );
    assert!(text(&about).contains(&running), "{about}");
    let invite = env.answer(command("invite"));
    assert_eq!(
        button(&invite, "Add to a server")["url"],
        format!(
            "https://discord.com/oauth2/authorize?client_id={APPLICATION_ID}&scope=bot+applications.commands&permissions=536870912&integration_type=0"
        )
    );
}

fn authorized(data: Value) -> Value {
    json!({
        "version": 1, "application_id": APPLICATION_ID, "type": 1,
        "event": { "type": "APPLICATION_AUTHORIZED", "timestamp": "2026-10-20T14:42:53.064834",
                   "data": data },
    })
}

fn send_event(env: &TestEnv, payload: Value) -> u16 {
    let key = TestKey::default();
    let request = key.signed(payload.to_string());
    testing::block_on(handle_event(
        &key.public_key(),
        Some(&request.signature),
        Some(&request.timestamp),
        &request.body,
        ComposureBot,
        env,
    ))
    .status()
}

#[test]
fn installs_are_logged_and_server_installers_welcomed() {
    let alice = json!({ "id": users::alice().id, "username": "alice", "discriminator": "0" });
    let env = TestEnv::default();
    let guild = json!({ "id": GUILD_ID, "name": "Rustaceans", "icon": null });
    assert_eq!(
        send_event(
            &env,
            authorized(
                json!({ "integration_type": 0, "user": alice, "scopes": ["bot"], "guild": guild })
            )
        ),
        204
    );
    env.run_background().unwrap();
    let sent = env.sent();
    assert_eq!(sent[0].1, format!("/channels/{LOG}/messages"));
    assert!(
        sent[0].2.contains("### Added to a server") && sent[0].2.contains("**Rustaceans**"),
        "{}",
        sent[0].2
    );
    let log: Value = serde_json::from_str(&sent[0].2).unwrap();
    assert_eq!(log["flags"], 1 << 15);
    assert_eq!(
        log["components"][0]["accent_color"],
        composure_bot::ui::SECONDARY
    );
    assert_eq!(sent[1].1, "/users/@me/channels");
    assert_eq!(sent[2].1, "/channels/77/messages");
    assert!(sent[2].2.contains("/notify setup"));

    // User installs are only logged
    let env = TestEnv::default();
    send_event(
        &env,
        authorized(
            json!({ "integration_type": 1, "user": alice, "scopes": ["applications.commands"] }),
        ),
    );
    env.run_background().unwrap();
    assert_eq!(env.paths(), [format!("/channels/{LOG}/messages")]);
}
