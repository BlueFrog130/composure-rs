//! Signed interaction fixtures for `tests/regression.rs`.
//!
//! Payloads follow the shapes Discord sends today (resolved data, member vs user, contexts,
//! autocomplete partial input, select values, modal rows/labels). Each case states the HTTP
//! status and, for 200s, the exact JSON [`RegressionBot`] must answer with.

#![allow(dead_code)]

mod regression_bot;

pub use regression_bot::RegressionBot;

use composure::testing::TestKey;
use serde_json::{Value, json};

pub const TIMESTAMP: &str = "1760000000";

const APP: &str = "1052322265397739523";
const GUILD: &str = "798662131062931547";
const CHANNEL: &str = "300000000000000001";
const ALICE: &str = "100000000000000001";
const BOB: &str = "100000000000000002";
const ROLE: &str = "200000000000000001";
const ATTACHMENT: &str = "400000000000000001";
const MESSAGE: &str = "500000000000000001";

pub fn public_key() -> String {
    TestKey::default().public_key()
}

pub fn sign(timestamp: &str, body: &[u8]) -> String {
    TestKey::default().sign(timestamp, body)
}

/// How a case's request is signed
#[derive(Clone, Copy, Debug)]
pub enum Signing {
    Valid,
    /// Signed, then the body is changed
    Tampered,
    /// No signature headers at all
    Missing,
}

pub struct Case {
    pub name: &'static str,
    pub body: Vec<u8>,
    pub signing: Signing,
    pub status: u16,
    /// Expected JSON body for 200 responses
    pub response: Option<Value>,
}

impl Case {
    fn ok(name: &'static str, payload: Value, response: Value) -> Self {
        Case {
            name,
            body: payload.to_string().into_bytes(),
            signing: Signing::Valid,
            status: 200,
            response: Some(response),
        }
    }

    fn message(name: &'static str, payload: Value, content: &str) -> Self {
        Self::ok(
            name,
            payload,
            json!({ "type": 4, "data": { "content": content } }),
        )
    }

    /// (signature, timestamp, body) to send
    pub fn request(&self) -> (Option<String>, Option<String>, Vec<u8>) {
        match self.signing {
            Signing::Valid => (
                Some(sign(TIMESTAMP, &self.body)),
                Some(TIMESTAMP.into()),
                self.body.clone(),
            ),
            Signing::Tampered => {
                let mut body = self.body.clone();
                body.extend_from_slice(b" ");
                (
                    Some(sign(TIMESTAMP, &self.body)),
                    Some(TIMESTAMP.into()),
                    body,
                )
            }
            Signing::Missing => (None, None, self.body.clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// Payload pieces
// ---------------------------------------------------------------------------

fn user(id: &str, username: &str, global_name: Option<&str>) -> Value {
    json!({
        "id": id,
        "username": username,
        "discriminator": "0",
        "global_name": global_name,
        "avatar": null,
        "avatar_decoration_data": null,
        "collectibles": null,
        "primary_guild": null,
        "public_flags": 0
    })
}

fn alice() -> Value {
    user(ALICE, "alice", Some("Alice"))
}

fn member(user: Value, nick: Option<&str>) -> Value {
    json!({
        "user": user,
        "nick": nick,
        "avatar": null,
        "banner": null,
        "roles": [ROLE],
        "joined_at": "2021-01-12T21:18:10.481000+00:00",
        "premium_since": null,
        "deaf": false,
        "mute": false,
        "flags": 0,
        "pending": false,
        "permissions": "2251799813685247",
        "communication_disabled_until": null,
        "unusual_dm_activity_until": null
    })
}

fn resolved_member(nick: &str) -> Value {
    json!({
        "nick": nick,
        "avatar": null,
        "banner": null,
        "roles": [],
        "joined_at": "2021-01-12T21:18:10.481000+00:00",
        "premium_since": null,
        "flags": 0,
        "pending": false,
        "permissions": "2251799813685247",
        "communication_disabled_until": null
    })
}

fn role() -> Value {
    json!({
        "id": ROLE,
        "name": "Mods",
        "color": 3447003,
        "colors": { "primary_color": 3447003, "secondary_color": null, "tertiary_color": null },
        "hoist": true,
        "icon": null,
        "unicode_emoji": null,
        "position": 3,
        "permissions": "1071698660929",
        "managed": false,
        "mentionable": true,
        "flags": 0
    })
}

fn text_channel() -> Value {
    json!({
        "id": CHANNEL,
        "type": 0,
        "guild_id": GUILD,
        "name": "general",
        "topic": null,
        "nsfw": false,
        "position": 1,
        "parent_id": null,
        "last_message_id": MESSAGE,
        "rate_limit_per_user": 0,
        "flags": 0,
        "permissions": "2251799813685247"
    })
}

fn attachment() -> Value {
    json!({
        "id": ATTACHMENT,
        "filename": "cat.png",
        "size": 12345,
        "url": "https://cdn.discordapp.com/ephemeral-attachments/1/2/cat.png",
        "proxy_url": "https://media.discordapp.net/ephemeral-attachments/1/2/cat.png",
        "content_type": "image/png",
        "width": 64,
        "height": 64,
        "ephemeral": true,
        "placeholder": "abc",
        "placeholder_version": 1
    })
}

fn message() -> Value {
    json!({
        "id": MESSAGE,
        "channel_id": CHANNEL,
        "type": 0,
        "content": "hello from alice",
        "author": alice(),
        "attachments": [],
        "embeds": [],
        "mentions": [],
        "mention_roles": [ROLE],
        "pinned": false,
        "mention_everyone": false,
        "tts": false,
        "timestamp": "2026-10-15T16:12:26.000000+00:00",
        "edited_timestamp": null,
        "flags": 0,
        "components": []
    })
}

/// Guild interaction envelope invoked by Alice
fn guild(t: u8, data: Value) -> Value {
    json!({
        "id": "1100173248714518568",
        "application_id": APP,
        "type": t,
        "data": data,
        "guild": { "id": GUILD, "locale": "en-US", "features": [] },
        "guild_id": GUILD,
        "guild_locale": "en-US",
        "channel": text_channel(),
        "channel_id": CHANNEL,
        "member": member(alice(), None),
        "token": "interaction-token",
        "version": 1,
        "app_permissions": "2248473465835073",
        "locale": "en-US",
        "entitlements": [],
        "entitlement_sku_ids": [],
        "authorizing_integration_owners": { "0": GUILD },
        "context": 0,
        "attachment_size_limit": 10485760
    })
}

/// Bot DM envelope for a user-installed command invoked by Alice
fn bot_dm(t: u8, data: Value) -> Value {
    json!({
        "id": "1100173248714518569",
        "application_id": APP,
        "type": t,
        "data": data,
        "channel": {
            "id": "300000000000000009",
            "type": 1,
            "flags": 0,
            "last_message_id": null,
            "recipients": [alice()]
        },
        "channel_id": "300000000000000009",
        "user": alice(),
        "token": "interaction-token",
        "version": 1,
        "app_permissions": "562949953601536",
        "locale": "en-US",
        "entitlements": [],
        "authorizing_integration_owners": { "1": ALICE },
        "context": 1,
        "attachment_size_limit": 10485760
    })
}

fn command(name: &str, options: Value) -> Value {
    json!({
        "id": "1052358444704862218",
        "name": name,
        "type": 1,
        "guild_id": GUILD,
        "options": options
    })
}

fn resolved_everything() -> Value {
    json!({
        "users": { ALICE: alice() },
        "members": { ALICE: resolved_member("Ali") },
        "roles": { ROLE: role() },
        "channels": { CHANNEL: text_channel() },
        "attachments": { ATTACHMENT: attachment() }
    })
}

// ---------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------

pub fn cases() -> Vec<Case> {
    let mut options = command(
        "regress-options",
        json!([
            { "name": "text", "type": 3, "value": "hello" },
            { "name": "count", "type": 4, "value": -3 },
            { "name": "color", "type": 3, "value": "red" },
            { "name": "size", "type": 4, "value": 9007199254740991_i64 },
            { "name": "ratio", "type": 10, "value": 1.25 },
            { "name": "factor", "type": 10, "value": 2 },
            { "name": "search", "type": 3, "value": "abc" },
            { "name": "flag", "type": 5, "value": true },
            { "name": "member", "type": 6, "value": ALICE },
            { "name": "channel", "type": 7, "value": CHANNEL },
            { "name": "role", "type": 8, "value": ROLE },
            { "name": "target", "type": 9, "value": ROLE },
            { "name": "file", "type": 11, "value": ATTACHMENT }
        ]),
    );
    options["resolved"] = resolved_everything();

    let mut ban = command(
        "regress-subcommands",
        json!([{
            "name": "admin",
            "type": 2,
            "options": [{
                "name": "ban",
                "type": 1,
                "options": [
                    { "name": "user", "type": 6, "value": ALICE },
                    { "name": "reason", "type": 3, "value": "spam" },
                    { "name": "days", "type": 4, "value": 7 }
                ]
            }]
        }]),
    );
    ban["resolved"] = json!({
        "users": { ALICE: alice() },
        "members": { ALICE: resolved_member("Ali") }
    });

    let mut echo = command(
        "regress-subcommands",
        json!([{
            "name": "echo",
            "type": 1,
            "options": [
                { "name": "message", "type": 3, "value": "hi there" },
                { "name": "where", "type": 7, "value": CHANNEL },
                { "name": "file", "type": 11, "value": ATTACHMENT }
            ]
        }]),
    );
    echo["resolved"] = json!({
        "channels": { CHANNEL: text_channel() },
        "attachments": { ATTACHMENT: attachment() }
    });

    let user_command = json!({
        "id": "1052358444704862219",
        "name": "Regress User",
        "type": 2,
        "guild_id": GUILD,
        "target_id": ALICE,
        "resolved": {
            "users": { ALICE: alice() },
            "members": { ALICE: resolved_member("Ali") }
        }
    });

    let message_command = json!({
        "id": "1052358444704862220",
        "name": "Regress Message",
        "type": 3,
        "guild_id": GUILD,
        "target_id": MESSAGE,
        "resolved": { "messages": { MESSAGE: message() } }
    });

    let modal_rows = json!({
        "custom_id": "feedback",
        "components": [
            { "type": 1, "id": 1, "components": [
                { "type": 4, "id": 2, "custom_id": "title", "value": "Hi" }
            ] },
            { "type": 1, "id": 3, "components": [
                { "type": 4, "id": 4, "custom_id": "body", "value": "Line 1\nLine 2" }
            ] }
        ]
    });

    let modal_labels = json!({
        "custom_id": "survey",
        "components": [
            { "type": 18, "id": 1, "component":
                { "type": 4, "id": 2, "custom_id": "title", "value": "Labelled" } },
            { "type": 18, "id": 3, "component":
                { "type": 3, "id": 4, "custom_id": "mood", "values": ["happy", "curious"] } },
            { "type": 10, "id": 5 }
        ]
    });

    vec![
        Case::ok(
            "ping",
            json!({
                "id": "1100173248714518570",
                "application_id": APP,
                "type": 1,
                "token": "interaction-token",
                "version": 1,
                "entitlements": [],
                "authorizing_integration_owners": {},
                "attachment_size_limit": 10485760
            }),
            json!({ "type": 1 }),
        ),
        Case::message(
            "every option type",
            guild(2, options),
            "regress-options text=hello count=-3 color=red size=9007199254740991 ratio=1.25 \
             factor=2 search=abc flag=true member=@Alice channel=#general role=@&Mods \
             target=@&Mods file=cat.png",
        ),
        Case::message(
            "subcommand group",
            guild(2, ban),
            "regress-subcommands admin ban user=@Alice reason=spam days=7",
        ),
        Case::message(
            "subcommand without options key",
            guild(
                2,
                command(
                    "regress-subcommands",
                    json!([{ "name": "ping", "type": 1 }]),
                ),
            ),
            "regress-subcommands ping",
        ),
        Case::message(
            "subcommand with empty options",
            guild(
                2,
                command(
                    "regress-subcommands",
                    json!([{ "name": "ping", "type": 1, "options": [] }]),
                ),
            ),
            "regress-subcommands ping",
        ),
        Case::message(
            "subcommand with channel and attachment",
            guild(2, echo),
            "regress-subcommands echo message=hi there where=#general file=cat.png",
        ),
        Case::message(
            "user command",
            guild(2, user_command),
            "Regress User -> Alice",
        ),
        Case::message(
            "message command",
            guild(2, message_command),
            "Regress Message -> hello from alice",
        ),
        Case::message(
            "user-installed command in bot DM",
            bot_dm(
                2,
                json!({
                    "id": "1052358444704862221",
                    "name": "regress-contexts",
                    "type": 1
                }),
            ),
            "regress-contexts by Alice in Some(BotDm) owners=1",
        ),
        Case::ok(
            "autocomplete string",
            guild(
                4,
                command(
                    "regress-options",
                    json!([
                        { "name": "text", "type": 3, "value": "hello" },
                        { "name": "search", "type": 3, "value": "ap", "focused": true }
                    ]),
                ),
            ),
            json!({ "type": 8, "data": { "choices": [
                { "name": "search: ap", "value": "ap" }
            ] } }),
        ),
        Case::ok(
            "autocomplete integer partial input",
            guild(
                4,
                command(
                    "regress-options",
                    json!([
                        { "name": "rank", "type": 4, "value": "12", "focused": true }
                    ]),
                ),
            ),
            json!({ "type": 8, "data": { "choices": [
                { "name": "rank: 12", "value": 12 }
            ] } }),
        ),
        Case::ok(
            "autocomplete integer unparseable input",
            guild(
                4,
                command(
                    "regress-options",
                    json!([
                        { "name": "rank", "type": 4, "value": "-", "focused": true }
                    ]),
                ),
            ),
            json!({ "type": 8, "data": { "choices": [] } }),
        ),
        Case::ok(
            "autocomplete number",
            guild(
                4,
                command(
                    "regress-options",
                    json!([
                        { "name": "weight", "type": 10, "value": "1.5", "focused": true }
                    ]),
                ),
            ),
            json!({ "type": 8, "data": { "choices": [
                { "name": "weight: 1.5", "value": 1.5 }
            ] } }),
        ),
        Case::ok(
            "autocomplete inside subcommand group",
            guild(
                4,
                command(
                    "regress-subcommands",
                    json!([{
                        "name": "admin", "type": 2, "options": [{
                            "name": "ban", "type": 1, "options": [
                                { "name": "reason", "type": 3, "value": "sp", "focused": true }
                            ]
                        }]
                    }]),
                ),
            ),
            json!({ "type": 8, "data": { "choices": [
                { "name": "reason: sp", "value": "sp" }
            ] } }),
        ),
        Case::message(
            "button",
            {
                let mut payload = guild(3, json!({ "custom_id": "confirm", "component_type": 2 }));
                payload["message"] = message();
                payload
            },
            "component confirm",
        ),
        Case::message(
            "string select",
            guild(
                3,
                json!({
                    "custom_id": "pick",
                    "component_type": 3,
                    "values": ["a", "b"]
                }),
            ),
            "component pick a,b",
        ),
        Case::message(
            "user select with resolved users",
            guild(
                3,
                json!({
                    "custom_id": "who",
                    "component_type": 5,
                    "values": [ALICE, BOB],
                    "resolved": {
                        "users": {
                            ALICE: alice(),
                            BOB: user(BOB, "bob", None)
                        }
                    }
                }),
            ),
            &format!("component who {ALICE},{BOB} users=Alice,bob"),
        ),
        Case::message(
            "modal with action rows",
            guild(5, modal_rows),
            "modal feedback title=Hi body=Line 1\nLine 2",
        ),
        Case::message(
            "modal with labels and select",
            guild(5, modal_labels),
            "modal survey title=Labelled body=- mood=happy,curious",
        ),
        Case::ok(
            "handler error becomes ephemeral message",
            guild(2, command("regress-fail", json!([]))),
            json!({ "type": 4, "data": {
                "content": "Something went wrong handling this interaction.",
                "flags": 64
            } }),
        ),
        Case {
            name: "tampered body",
            body: guild(2, command("regress-options", json!([])))
                .to_string()
                .into_bytes(),
            signing: Signing::Tampered,
            status: 401,
            response: None,
        },
        Case {
            name: "missing signature headers",
            body: guild(2, command("regress-options", json!([])))
                .to_string()
                .into_bytes(),
            signing: Signing::Missing,
            status: 401,
            response: None,
        },
        Case {
            name: "signed but not an interaction",
            body: br#"{"type": 99}"#.to_vec(),
            signing: Signing::Valid,
            status: 400,
            response: None,
        },
    ]
}
