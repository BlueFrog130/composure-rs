//! Signed interactions, events and the cron work through the bot on the host. `TestEnv` stands
//! in for `Worker<Config>`: a map for KV, and a Transport answering Discord, GitHub and
//! crates.io with canned bodies while recording every request.

use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

use composure::{
    adapter::{handle_event, handle_interaction},
    command::{BackgroundWork, CommandSetDefinition, HandlerError, HasData, Platform},
    rest::{BotClient, Method, RawResponse, Request, Transport, TransportError},
    types::Snowflake,
};
use composure_bot::{ComposureBot, Config, Store, notify};
use ed25519_dalek::{Signer, SigningKey};
use futures::executor::block_on;
use serde_json::{Value, json};

const KEY: [u8; 32] = [7; 32];
const TIMESTAMP: &str = "1760000000";
const APP: &str = "1052322265397739523";
const GUILD: &str = "798662131062931547";
const CHANNEL: &str = "300000000000000001";
const NEWS: &str = "300000000000000002";
const ANNOUNCE: &str = "600000000000000001";
const FEEDBACK: &str = "600000000000000002";
const LOG: &str = "600000000000000003";
const ALICE: &str = "100000000000000001";
const BOB: &str = "100000000000000002";
const ORIGINAL: &str = "/webhooks/1052322265397739523/token/messages/@original";

static CONFIG: LazyLock<Config> = LazyLock::new(|| {
    let vars = HashMap::from([
        ("ANNOUNCE_CHANNEL_ID", ANNOUNCE),
        ("FEEDBACK_CHANNEL_ID", FEEDBACK),
        ("LOG_CHANNEL_ID", LOG),
        ("GITHUB_TOKEN", "gh-token"),
    ]);
    Config::from_lookup(|name| vars.get(name).map(|value| value.to_string())).unwrap()
});

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
        "id": id, "channel_id": channel, "author": alice(), "content": "",
        "timestamp": "2026-10-20T00:00:00.000000+00:00", "edited_timestamp": null, "tts": false,
        "mention_everyone": false, "mentions": [], "mention_roles": [], "attachments": [],
        "embeds": [], "pinned": false, "type": 0,
    })
}

fn follower(id: &str, channel: &str) -> Value {
    json!({
        "id": id, "type": 2, "guild_id": GUILD, "channel_id": channel, "name": "Composure #news",
        "avatar": null, "application_id": null,
        "source_guild": { "id": "1", "name": "Composure", "icon": null },
        "source_channel": { "id": ANNOUNCE, "name": "releases" },
    })
}

/// Discord, GitHub and crates.io, recording requests
#[derive(Default)]
struct World {
    requests: Mutex<Vec<Request>>,
    /// This guild's webhooks
    webhooks: Mutex<Vec<Value>>,
    /// Answer webhook requests with 403
    forbidden: Mutex<bool>,
}

impl Transport for World {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        self.requests.lock().unwrap().push(request.clone());
        let ok = |body: Value| RawResponse {
            status: 200,
            headers: vec![("content-type".into(), "application/json".into())],
            body: body.to_string().into_bytes(),
        };
        if request.url.starts_with("https://api.github.com/") {
            return Ok(ok(github_releases()));
        }
        if let Some(name) = request.url.strip_prefix("https://crates.io/api/v1/crates/") {
            return Ok(ok(json!({ "crate": {
                "name": name, "description": "HTTP-only Discord bots", "max_version": "0.2.0-rc.1",
                "max_stable_version": "0.1.2", "downloads": 12345, "recent_downloads": 678,
                "updated_at": "2026-10-20T17:05:02.000000+00:00",
            } })));
        }
        let path = request.url.rsplit("/v10").next().unwrap();
        let webhooks = path.contains("/webhooks") && !path.starts_with("/webhooks/1052");
        if webhooks && *self.forbidden.lock().unwrap() {
            return Ok(RawResponse {
                status: 403,
                headers: vec![("content-type".into(), "application/json".into())],
                body: br#"{"message":"Missing Permissions","code":50013}"#.to_vec(),
            });
        }
        Ok(match (request.method, path) {
            (Method::Get, p) if p == format!("/guilds/{GUILD}/webhooks") => {
                ok(Value::Array(self.webhooks.lock().unwrap().clone()))
            }
            (Method::Post, p) if p.ends_with("/followers") => {
                ok(json!({ "channel_id": ANNOUNCE, "webhook_id": "800000000000000009" }))
            }
            (Method::Delete, _) => RawResponse {
                status: 204,
                headers: Vec::new(),
                body: Vec::new(),
            },
            (Method::Post, "/users/@me/channels") => ok(json!({ "id": "77", "type": 1 })),
            (Method::Post, p) if p.starts_with("/channels/") => {
                let channel = p.split('/').nth(2).unwrap();
                ok(message("700000000000000001", channel))
            }
            _ => ok(json!({})),
        })
    }

    async fn sleep(&self, _duration: Duration) {}
}

#[derive(Clone, Default)]
struct TestEnv {
    world: Arc<World>,
    kv: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    work: Arc<Mutex<Vec<BackgroundWork>>>,
}

impl HasData for TestEnv {
    type Data = Config;

    fn data(&self) -> &Config {
        &CONFIG
    }
}

impl Platform for TestEnv {
    type Transport = Arc<World>;

    fn transport(&self) -> Arc<World> {
        self.world.clone()
    }

    fn spawn(&self, work: BackgroundWork) {
        self.work.lock().unwrap().push(work);
    }

    fn bot_token(&self) -> Option<String> {
        Some("bot-token".into())
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
        let work: Vec<_> = self.work.lock().unwrap().drain(..).collect();
        work.into_iter().try_for_each(block_on)
    }

    /// (method, URL without Discord's base, body) of every request sent
    fn sent(&self) -> Vec<(Method, String, String)> {
        self.world
            .requests
            .lock()
            .unwrap()
            .iter()
            .map(|r| {
                (
                    r.method,
                    r.url.rsplit("/v10").next().unwrap().to_string(),
                    String::from_utf8_lossy(r.body.as_deref().unwrap_or_default()).into_owned(),
                )
            })
            .collect()
    }

    fn paths(&self) -> Vec<String> {
        self.sent().into_iter().map(|(_, path, _)| path).collect()
    }

    fn github_calls(&self) -> usize {
        self.paths()
            .iter()
            .filter(|path| path.starts_with("https://api.github.com/"))
            .count()
    }

    fn send(&self, payload: Value) -> (u16, Value) {
        let body = payload.to_string().into_bytes();
        let key = SigningKey::from_bytes(&KEY);
        let signature = hex::encode(key.sign(&[TIMESTAMP.as_bytes(), &body].concat()).to_bytes());
        let outcome = block_on(handle_interaction(
            &hex::encode(key.verifying_key().as_bytes()),
            Some(&signature),
            Some(TIMESTAMP),
            &body,
            &ComposureBot,
            self,
        ));
        let response = outcome
            .response()
            .map(|r| serde_json::to_value(r).unwrap())
            .unwrap_or_default();
        (outcome.status(), response)
    }

    /// The response to `payload`, which must succeed
    fn answer(&self, payload: Value) -> Value {
        let (status, response) = self.send(payload);
        assert_eq!(status, 200, "{response}");
        response
    }
}

fn alice() -> Value {
    user(ALICE, "alice")
}

fn user(id: &str, name: &str) -> Value {
    json!({ "id": id, "username": name, "discriminator": "0", "global_name": null, "avatar": null })
}

fn interaction(kind: u8, by: Value, data: Value) -> Value {
    json!({
        "id": "1100173248714518568", "application_id": APP, "type": kind, "token": "token",
        "version": 1, "channel_id": CHANNEL, "guild_id": GUILD,
        "member": { "user": by, "roles": [], "joined_at": "2021-01-12T21:18:10.481000+00:00",
                    "deaf": false, "mute": false, "flags": 0, "permissions": "32" },
        "data": data,
    })
}

fn command(name: &str, options: Value) -> Value {
    interaction(
        2,
        alice(),
        json!({ "id": "1", "name": name, "type": 1, "options": options }),
    )
}

fn autocomplete(name: &str, option: &str, typed: &str) -> Value {
    interaction(
        4,
        alice(),
        json!({ "id": "1", "name": name, "type": 1, "options": [
            { "name": option, "type": 3, "value": typed, "focused": true },
        ] }),
    )
}

fn click(by: Value, custom_id: &str, values: &[&str]) -> Value {
    interaction(
        3,
        by,
        json!({ "custom_id": custom_id, "component_type": 2, "values": values }),
    )
}

/// Every component object in a response, depth first
fn components(response: &Value) -> Vec<&Value> {
    fn walk<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
        match value {
            Value::Object(map) => {
                if map.contains_key("type")
                    && (map.contains_key("custom_id")
                        || map.contains_key("url")
                        || map.contains_key("content"))
                {
                    out.push(value);
                }
                map.values().for_each(|v| walk(v, out));
            }
            Value::Array(items) => items.iter().for_each(|v| walk(v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(&response["data"]["components"], &mut out);
    out
}

/// The text displays' content, joined
fn text(response: &Value) -> String {
    components(response)
        .into_iter()
        .filter(|c| c["type"] == 10)
        .filter_map(|c| c["content"].as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The component whose label or custom_id prefix is `name`
fn button<'a>(response: &'a Value, label: &str) -> &'a Value {
    components(response)
        .into_iter()
        .find(|c| c["label"] == label)
        .unwrap_or_else(|| panic!("no {label} in {response}"))
}

fn custom_id<'a>(response: &'a Value, label: &str) -> &'a str {
    button(response, label)["custom_id"].as_str().unwrap()
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
    let response = env.answer(command("changelog", json!([])));
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
    let select = components(&response)
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
    let older = custom_id(&response, "Older ▶").to_string();
    assert_eq!(older, format!("cl:older:{ALICE}:::0.1.1"));
    let turned = env.answer(click(alice(), &older, &[]));
    assert_eq!(turned["type"], 7);
    assert!(text(&turned).contains("## composure v0.1.1"));
    assert!(text(&turned).contains("2 of 3"));

    // Anyone else gets a copy of their own
    let copy = env.answer(click(user(BOB, "bob"), &older, &[]));
    assert_eq!(copy["type"], 4);
    assert_eq!(copy["data"]["flags"].as_u64().unwrap() & 64, 64);
    assert!(custom_id(&copy, "Older ▶").starts_with(&format!("cl:older:{BOB}:")));

    // The jump menu
    let select = components(&turned)
        .into_iter()
        .find(|c| c["type"] == 3)
        .unwrap();
    let jump = select["custom_id"].as_str().unwrap();
    let mut payload = click(alice(), jump, &["0.1.0"]);
    payload["data"]["component_type"] = json!(3);
    let jumped = env.answer(payload);
    assert!(text(&jumped).contains("## composure v0.1.0"));
    assert_eq!(button(&jumped, "Older ▶")["disabled"], true);

    // GitHub was asked once; every page after came from the store
    assert_eq!(env.github_calls(), 1);
    let request = env.world.requests.lock().unwrap()[0].clone();
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
        response["data"]["choices"],
        json!([
            { "name": "v0.1.2 (2026-10-20)", "value": "0.1.2" },
            { "name": "v0.1.1 (2026-10-20)", "value": "0.1.1" },
            { "name": "v0.1.0 (2026-10-20)", "value": "0.1.0" },
        ])
    );

    let response = env.answer(command(
        "changelog",
        json!([{ "name": "version", "type": 3, "value": "v0.1.1" }]),
    ));
    assert!(text(&response).contains("## composure v0.1.1"));

    let (status, response) = env.send(command(
        "changelog",
        json!([{ "name": "version", "type": 3, "value": "9.9.9" }]),
    ));
    assert_eq!(status, 200);
    assert_eq!(response["data"]["flags"], 64);
    assert!(
        response.to_string().contains("There is no release `9.9.9`"),
        "{response}"
    );
    // The framework's error embed, in the error color with no branding containers lack
    let error = &response["data"]["embeds"][0];
    assert_eq!(error["color"], composure_bot::THEME.error);
    assert!(
        error["author"].is_null() && error["footer"].is_null(),
        "{error}"
    );
}

#[test]
fn compare_pages_through_a_range() {
    let env = TestEnv::default();
    let response = env.answer(command(
        "compare",
        json!([{ "name": "from", "type": 3, "value": "0.1.1" }]),
    ));
    let page = text(&response);
    assert!(
        page.contains("Changes from v0.1.1 to v0.1.2: 2 releases"),
        "{page}"
    );
    assert!(page.contains("1 of 2"));
    let older = custom_id(&response, "Older ▶");
    assert_eq!(older, format!("cl:older:{ALICE}:0.1.1:0.1.2:0.1.1"));
    let last = env.answer(click(alice(), older, &[]));
    assert!(text(&last).contains("2 of 2"));
    assert_eq!(button(&last, "Older ▶")["disabled"], true);
}

#[test]
fn explain_release_reads_the_version_from_a_message() {
    let env = TestEnv::default();
    let mut target = message("500000000000000001", CHANNEL);
    target["embeds"] = json!([{ "title": "composure v0.1.1 is out" }]);
    let response = env.answer(interaction(
        2,
        alice(),
        json!({ "id": "1", "name": "Explain release", "type": 3, "target_id": "500000000000000001",
                "resolved": { "messages": { "500000000000000001": target } } }),
    ));
    assert_eq!(response["data"]["flags"].as_u64().unwrap() & 64, 64);
    assert!(text(&response).contains("## composure v0.1.1"));
}

#[test]
fn notify_follows_lists_and_removes() {
    let env = TestEnv::default();
    // Channel Options arrive with the channel in `resolved`
    let mut payload_with_channel = command("notify", json!([]));
    payload_with_channel["data"]["resolved"] = json!({ "channels": { NEWS: { "id": NEWS, "type": 0, "name": "news", "permissions": "0" } } });
    let with_channel = |options: Value| {
        let mut payload = payload_with_channel.clone();
        payload["data"]["options"] = options;
        payload
    };

    let response = env.answer(with_channel(
        json!([{ "name": "setup", "type": 1, "options": [
        { "name": "channel", "type": 7, "value": NEWS },
    ] }]),
    ));
    assert!(
        response
            .to_string()
            .contains(&format!("announced in <#{NEWS}>")),
        "{response}"
    );
    let sent = env.sent();
    assert_eq!(sent[0].1, format!("/guilds/{GUILD}/webhooks"));
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
        "id": "800000000000000010", "type": 1, "guild_id": GUILD, "channel_id": CHANNEL,
        "name": "Someone else's", "avatar": null, "application_id": null,
    }));
    let response = env.answer(with_channel(
        json!([{ "name": "setup", "type": 1, "options": [
        { "name": "channel", "type": 7, "value": NEWS },
    ] }]),
    ));
    assert!(
        response
            .to_string()
            .contains("already gets release announcements")
    );
    let response = env.answer(with_channel(json!([{ "name": "status", "type": 1 }])));
    assert!(
        response.to_string().contains(&format!("go to <#{NEWS}>.")),
        "{response}"
    );

    let before = env.sent().len();
    let response = env.answer(with_channel(json!([{ "name": "remove", "type": 1 }])));
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
    *env.world.forbidden.lock().unwrap() = true;
    let (status, response) = env.send(command("notify", json!([{ "name": "status", "type": 1 }])));
    assert_eq!(status, 200);
    assert_eq!(response["data"]["flags"], 64);
    assert!(
        response.to_string().contains("Manage Webhooks"),
        "{response}"
    );
}

#[test]
fn notify_preview_shows_the_announcement() {
    let env = TestEnv::default();
    let response = env.answer(command("notify", json!([{ "name": "preview", "type": 1 }])));
    // Components V2, ephemeral
    assert_eq!(response["data"]["flags"], 64 | (1 << 15));
    let container = &response["data"]["components"][0];
    assert_eq!(container["accent_color"], 0x7c5cff);
    let page = text(&response);
    assert!(page.contains("## composure v0.1.2 is out"), "{page}");
    assert!(page.contains("Released 2026-10-20"), "{page}");
    let links: Vec<_> = components(&response)
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
    let bot = BotClient::new(
        env.world.clone(),
        "bot-token",
        Snowflake::from(1052322265397739523u64),
    );
    let announce = || block_on(notify::announce_new(&bot, &env.world, &env, &CONFIG)).unwrap();

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
    env.world.requests.lock().unwrap().clear();

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
    assert_eq!(first["components"][0]["accent_color"], 0x7c5cff);
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
    let first = &response["data"]["choices"][0];
    assert_eq!(first["name"], "Views › Inline handlers");

    let picked = env.answer(command(
        "docs",
        json!([{ "name": "topic", "type": 3, "value": first["value"] }]),
    ));
    assert!(text(&picked).contains("### Views › Inline handlers"));
    assert_eq!(
        button(&picked, "Open")["url"],
        "https://bluefrog130.github.io/composure-rs/views.html#inline-handlers"
    );

    // Typed text without picking a suggestion is searched
    let typed = env.answer(command(
        "docs",
        json!([{ "name": "topic", "type": 3, "value": "rsx" }]),
    ));
    assert!(text(&typed).contains("### rsx!"), "{typed}");
}

#[test]
fn crate_stats_are_deferred_and_cached() {
    let env = TestEnv::default();
    let response = env.answer(command("crate", json!([])));
    assert_eq!(response, json!({ "type": 5 }));
    env.run_background().unwrap();
    let (method, path, body) = env.sent().pop().unwrap();
    assert_eq!((method, path.as_str()), (Method::Patch, ORIGINAL));
    assert!(body.contains("## composure 0.1.2"), "{body}");
    assert!(
        body.contains("**12,345** downloads · **678** in the last 90 days"),
        "{body}"
    );

    env.answer(command("crate", json!([])));
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
    let response = env.answer(command("feedback", json!([])));
    assert_eq!(response["type"], 9);
    assert_eq!(response["data"]["custom_id"], "fb:send");

    let label = |child: Value| json!({ "type": 18, "id": 1, "component": child });
    let submit = interaction(
        5,
        alice(),
        json!({ "custom_id": "fb:send", "components": [
            label(json!({ "type": 21, "custom_id": "kind", "value": "Bug" })),
            label(json!({ "type": 4, "custom_id": "title", "value": "Crash on start" })),
            label(json!({ "type": 4, "custom_id": "details", "value": "It panics." })),
        ] }),
    );
    let response = env.answer(submit);
    assert_eq!(response["data"]["flags"].as_u64().unwrap() & 64, 64);
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
        body.contains("### [Bug] Crash on start") && body.contains(&format!("<@{ALICE}>")),
        "{body}"
    );
    // A Components V2 message in the error tone: bugs stand out
    let report: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(report["flags"], 1 << 15);
    assert_eq!(
        report["components"][0]["accent_color"],
        composure_bot::THEME.error
    );
}

#[test]
fn about_and_invite() {
    let env = TestEnv::default();
    let about = env.answer(command("about", json!([])));
    let running = format!(
        "Running composure {}",
        composure_bot::build_info::COMPOSURE_VERSION
    );
    assert!(text(&about).contains(&running), "{about}");
    let invite = env.answer(command("invite", json!([])));
    assert_eq!(
        button(&invite, "Add to a server")["url"],
        format!(
            "https://discord.com/oauth2/authorize?client_id={APP}&scope=bot+applications.commands&permissions=536870912&integration_type=0"
        )
    );
}

fn authorized(data: Value) -> Value {
    json!({
        "version": 1, "application_id": APP, "type": 1,
        "event": { "type": "APPLICATION_AUTHORIZED", "timestamp": "2026-10-20T14:42:53.064834",
                   "data": data },
    })
}

fn send_event(env: &TestEnv, payload: Value) -> u16 {
    let body = payload.to_string().into_bytes();
    let key = SigningKey::from_bytes(&KEY);
    let signature = hex::encode(key.sign(&[TIMESTAMP.as_bytes(), &body].concat()).to_bytes());
    block_on(handle_event(
        &hex::encode(key.verifying_key().as_bytes()),
        Some(&signature),
        Some(TIMESTAMP),
        &body,
        ComposureBot,
        env,
    ))
    .status()
}

#[test]
fn installs_are_logged_and_server_installers_welcomed() {
    let env = TestEnv::default();
    let guild = json!({ "id": GUILD, "name": "Rustaceans", "icon": null });
    assert_eq!(
        send_event(
            &env,
            authorized(
                json!({ "integration_type": 0, "user": alice(), "scopes": ["bot"], "guild": guild })
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
            json!({ "integration_type": 1, "user": alice(), "scopes": ["applications.commands"] }),
        ),
    );
    env.run_background().unwrap();
    assert_eq!(env.paths(), [format!("/channels/{LOG}/messages")]);
}
