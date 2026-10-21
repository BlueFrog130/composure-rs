//! Allocation and time per interaction path through the example `Bot`: parse, dispatch, and
//! serialize the response (signature checking is measured separately, since Workers use Web
//! Crypto). Payloads carry what Discord sends in a guild: channel, member, entitlements, and the
//! message a component is attached to.
//!
//! `paths_respond_within_budget` runs offline and fails if a path allocates more than its budget.
//! For timings: `cargo test -p composure_example_bot --release --test perf -- --ignored --nocapture`

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
    sync::LazyLock,
    time::{Duration, Instant},
};

use composure::{
    adapter::{handle_interaction, handle_interaction_with_key},
    auth::PublicKey,
};
use composure::{
    command::{BackgroundWork, HasData, Platform, dispatch},
    rest::{RawResponse, Request, Transport, TransportError},
    types::Interaction,
};
use composure_example_bot::{Bot, Config};
use ed25519_dalek::{Signer, SigningKey};
use futures::executor::block_on;
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// Per-thread allocation counting
// ---------------------------------------------------------------------------

struct Counting;

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}

fn count(bytes: usize) {
    let _ = ALLOCS.try_with(|a| a.set(a.get() + 1));
    let _ = BYTES.try_with(|b| b.set(b.get() + bytes));
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(size);
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// (allocations, bytes) made by `f` on this thread
fn allocations<T>(f: impl FnOnce() -> T) -> (usize, usize, T) {
    let (a, b) = (ALLOCS.get(), BYTES.get());
    let out = f();
    (ALLOCS.get() - a, BYTES.get() - b, out)
}

// ---------------------------------------------------------------------------
// State stand-in
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct NoTransport;

impl Transport for NoTransport {
    async fn send(&self, _request: &Request) -> Result<RawResponse, TransportError> {
        Err(TransportError::new("offline"))
    }

    async fn sleep(&self, _duration: Duration) {}
}

/// Drops Background Work unrun: only the cost of answering is measured
#[derive(Clone)]
struct Env;

static CONFIG: LazyLock<Config> = LazyLock::new(|| Config {
    greeting: "Howdy".into(),
});

impl HasData for Env {
    type Data = Config;

    fn data(&self) -> &Config {
        &CONFIG
    }
}

impl Platform for Env {
    type Transport = NoTransport;

    fn transport(&self) -> NoTransport {
        NoTransport
    }

    fn spawn(&self, work: BackgroundWork) {
        drop(work);
    }

    fn bot_token(&self) -> Option<&str> {
        Some("bot-token")
    }
}

// ---------------------------------------------------------------------------
// Payloads
// ---------------------------------------------------------------------------

const APP: &str = "1052322265397739523";
const GUILD: &str = "798662131062931547";
const CHANNEL: &str = "941169456686723122";
const ALICE: &str = "282265607313817601";

fn user(id: &str, name: &str) -> Value {
    json!({
        "id": id, "username": name, "discriminator": "0", "global_name": name,
        "avatar": "fa82e15e24ee16c9fcbf8dd34d10b4cc", "avatar_decoration_data": null,
        "collectibles": null, "primary_guild": null, "public_flags": 0,
    })
}

fn guild(t: u8, data: Value) -> Value {
    json!({
        "app_permissions": "2248473465835073", "application_id": APP,
        "authorizing_integration_owners": { "0": GUILD },
        "channel": {
            "flags": 0, "guild_id": GUILD, "id": CHANNEL, "last_message_id": "1100155827400229026",
            "name": "bot-stuff", "nsfw": false, "parent_id": "798662131678969866",
            "permissions": "140737488355327", "position": 1, "rate_limit_per_user": 0,
            "topic": null, "type": 0,
        },
        "channel_id": CHANNEL, "context": 0, "data": data,
        "entitlement_sku_ids": [], "entitlements": [],
        "guild": { "id": GUILD, "locale": "en-US", "features": [] },
        "guild_id": GUILD, "guild_locale": "en-US", "id": "1100173248714518568", "locale": "en-US",
        "member": {
            "avatar": null, "banner": null, "communication_disabled_until": null, "deaf": false,
            "flags": 0, "joined_at": "2021-01-12T21:18:10.481000+00:00", "mute": false,
            "nick": null, "pending": false, "permissions": "140737488355327",
            "premium_since": null, "roles": ["943607715639484456", "943607715639484457"],
            "user": user(ALICE, "alice"),
        },
        "token": "aW50ZXJhY3Rpb246MTEwMDE3MzI0ODcxNDUxODU2ODppVTFuSkNSbndrZ01Na3RCWk81MVhTWkdSbk8yTlBaM1U3Z3JlckR4YUZJMTZFTm9wc21nZnlaSnN4ZUZCTTd0Q0Jzc09ac3BHV1E1MGlBZGZnZzh0NDJmTElIcTB1M0FZQTJPS1BxcG1GTEtZUjNDWWFEamhEeTRPMWZnS0R4dQ",
        "type": t, "version": 1, "attachment_size_limit": 10485760,
    })
}

fn command(name: &str, options: Value) -> Value {
    guild(
        2,
        json!({ "id": "1052358444704862218", "name": name, "type": 1, "guild_id": GUILD, "options": options }),
    )
}

/// The bot's message a component sits on
fn message(flags: u32, components: Value) -> Value {
    json!({
        "id": "1100155827400229026", "channel_id": CHANNEL, "author": user(APP, "composure"),
        "content": if flags == 0 { "Clicked 4 times" } else { "" },
        "timestamp": "2026-10-15T16:12:26.000000+00:00", "edited_timestamp": null,
        "tts": false, "mention_everyone": false, "mentions": [], "mention_roles": [],
        "attachments": [], "embeds": [], "pinned": false, "type": 20, "flags": flags,
        "application_id": APP, "webhook_id": APP,
        "interaction_metadata": {
            "id": "1100173248714518000", "type": 2, "name": "counter", "command_type": 1,
            "user": user(ALICE, "alice"), "authorizing_integration_owners": { "0": GUILD },
        },
        "components": components,
    })
}

fn click(custom_id: &str, component_type: u8, values: Option<Value>, message: Value) -> Value {
    let mut data = json!({ "custom_id": custom_id, "component_type": component_type, "id": 3 });
    if let Some(values) = values {
        data["values"] = values;
    }
    let mut payload = guild(3, data);
    payload["message"] = message;
    payload
}

fn button(label: &str, custom_id: &str, id: u32) -> Value {
    json!({ "type": 2, "id": id, "style": 1, "label": label, "custom_id": custom_id })
}

/// The `card` command's Components V2 message
fn card_message() -> Value {
    message(
        1 << 15,
        json!([{ "type": 17, "id": 1, "accent_color": 16106215, "components": [
            { "type": 9, "id": 2, "components": [
                { "type": 10, "id": 3, "content": "## Components V2" },
                { "type": 10, "id": 4, "content": "Sections, separators, galleries and files in a container." },
            ], "accessory": { "type": 2, "id": 5, "style": 5, "label": "Docs",
                "url": "https://discord.com/developers/docs/components/reference" } },
            { "type": 14, "id": 6, "divider": true, "spacing": 1 },
            { "type": 10, "id": 7, "content": "-# 2 likes" },
            { "type": 1, "id": 8, "components": [button("Like", "card:like:2", 9), button("Review", "card:review:2", 10)] },
        ] }]),
    )
}

fn row(components: Value) -> Value {
    json!([{ "type": 1, "id": 1, "components": components }])
}

struct Case {
    name: &'static str,
    body: Vec<u8>,
    /// Checks the serialized response
    expect: fn(&Value),
    /// Most allocations parse + dispatch + serialize may make
    budget: usize,
}

fn cases() -> Vec<Case> {
    let label = |child: Value| json!({ "type": 18, "id": 1, "component": child });
    let case = |name, payload: Value, expect, budget| Case {
        name,
        body: payload.to_string().into_bytes(),
        expect,
        budget,
    };
    vec![
        case(
            "command /echo (2 options)",
            command(
                "echo",
                json!([
                    { "name": "text", "type": 3, "value": "hi" },
                    { "name": "times", "type": 4, "value": 3 },
                ]),
            ),
            |r| assert_eq!(r["data"]["content"], "hi hi hi"),
            28,
        ),
        case(
            "command /greet (State)",
            command("greet", json!([])),
            |r| assert_eq!(r["data"]["content"], "Howdy from Composure!"),
            23,
        ),
        case(
            "command /whoami (inline REST)",
            command("whoami", json!([])),
            // Offline, so the request fails: the cost of the client and the error answer, with the
            // error's cause kept for the logs
            |r| assert_eq!(r["data"]["flags"], 64),
            28,
        ),
        case(
            "autocomplete /fruit",
            {
                let mut payload = command(
                    "fruit",
                    json!([
                        { "name": "name", "type": 3, "value": "Bl", "focused": true },
                    ]),
                );
                payload["type"] = json!(4);
                payload
            },
            |r| {
                assert_eq!(
                    r,
                    &json!({ "type": 8, "data": { "choices": [{ "name": "blueberry", "value": "blueberry" }] } })
                )
            },
            30,
        ),
        case(
            "button counter (classic message)",
            click(
                "counter:4",
                2,
                None,
                message(0, row(json!([button("+1", "counter:4", 2)]))),
            ),
            |r| {
                assert_eq!(r["type"], 7);
                assert_eq!(r["data"]["content"], "Clicked 5 times");
            },
            32,
        ),
        case(
            "button like (V2 card View)",
            click("card:like:2", 2, None, card_message()),
            |r| {
                assert_eq!(r["type"], 7);
                assert_eq!(
                    r["data"]["components"][0]["components"][2]["content"],
                    "-# 3 likes"
                );
            },
            46,
        ),
        case(
            "button review (opens a modal View)",
            click("card:review:2", 2, None, card_message()),
            |r| {
                assert_eq!(r["type"], 9);
                assert_eq!(r["data"]["custom_id"], "review:send:2");
            },
            45,
        ),
        case(
            "button roll (deferred update)",
            click(
                "roll:2",
                2,
                None,
                message(0, row(json!([button("Roll", "roll:2", 2)]))),
            ),
            |r| assert_eq!(r, &json!({ "type": 6 })),
            26,
        ),
        case(
            "select colors",
            click(
                "colors",
                3,
                Some(json!(["red", "blue"])),
                message(
                    0,
                    row(
                        json!([{ "type": 3, "id": 2, "custom_id": "colors", "min_values": 1, "max_values": 3,
                    "options": [
                        { "label": "Red", "value": "red" }, { "label": "Green", "value": "green" },
                        { "label": "Blue", "value": "blue" },
                    ] }]),
                    ),
                ),
            ),
            |r| assert_eq!(r["data"]["content"], "You picked red, blue"),
            28,
        ),
        case(
            "modal feedback",
            guild(
                5,
                json!({ "custom_id": "feedback", "components": [
                label(json!({ "type": 4, "custom_id": "topic", "value": "Docs" })),
                label(json!({ "type": 21, "custom_id": "kind", "value": "Idea" })),
                label(json!({ "type": 4, "custom_id": "details", "value": "Needs examples" })),
                label(json!({ "type": 19, "custom_id": "screenshots", "values": [] })),
                label(json!({ "type": 23, "custom_id": "contact", "value": true })),
            ] }),
            ),
            |r| {
                assert_eq!(
                    r["data"]["content"],
                    "Thanks! Idea **Docs**: Needs examples (0 screenshots); we'll be in touch"
                )
            },
            48,
        ),
    ]
}

/// Parse, dispatch, and serialize, as the Adapter does after the signature check
fn answer(body: &[u8], env: &Env) -> Vec<u8> {
    let interaction: Interaction = serde_json::from_slice(body).unwrap();
    // Errors are answered as the Adapters answer them
    let response =
        block_on(dispatch(interaction, &Bot, env)).unwrap_or_else(|error| error.response());
    serde_json::to_vec(&response).unwrap()
}

#[test]
fn paths_respond_within_budget() {
    let env = Env;
    let mut over = Vec::new();
    for case in cases() {
        let (allocs, _, response) = allocations(|| answer(&case.body, &env));
        (case.expect)(&serde_json::from_slice(&response).unwrap());
        if allocs > case.budget {
            over.push(format!(
                "{}: {allocs} allocations, budget {}",
                case.name, case.budget
            ));
        }
    }
    assert!(over.is_empty(), "\n{}", over.join("\n"));
}

fn time(iterations: u32, mut f: impl FnMut()) -> f64 {
    f();
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    start.elapsed().as_secs_f64() * 1e6 / f64::from(iterations)
}

#[test]
#[ignore = "timings; run with --release --ignored --nocapture"]
fn report() {
    let env = Env;
    println!(
        "\n{:<34} {:>6} | {:>12} {:>8} {:>8} | {:>12} {:>8}",
        "path", "body B", "parse allocs", "bytes", "us", "total allocs", "us"
    );
    for case in cases() {
        let body = &case.body;
        let (parse_allocs, parse_bytes, _) =
            allocations(|| serde_json::from_slice::<Interaction>(body).unwrap());
        let (allocs, _, _) = allocations(|| answer(body, &env));
        let parse_us = time(5_000, || {
            black_box(serde_json::from_slice::<Interaction>(black_box(body)).unwrap());
        });
        let total_us = time(5_000, || {
            black_box(answer(black_box(body), &env));
        });
        println!(
            "{:<34} {:>6} | {parse_allocs:>12} {parse_bytes:>8} {parse_us:>8.2} | {allocs:>12} {total_us:>8.2}",
            case.name,
            body.len(),
        );
    }

    // Full Adapter path, signature included (ed25519-dalek on the host)
    let key = SigningKey::from_bytes(&[7; 32]);
    let public_key = hex::encode(key.verifying_key().as_bytes());
    let body = &cases()[0].body;
    let signature = hex::encode(
        key.sign(&[b"1760000000".as_slice(), body].concat())
            .to_bytes(),
    );
    let (allocs, _, _) = allocations(|| {
        block_on(handle_interaction(
            &public_key,
            Some(&signature),
            Some("1760000000"),
            body,
            &Bot,
            &env,
        ))
    });
    let us = time(2_000, || {
        black_box(block_on(handle_interaction(
            &public_key,
            Some(&signature),
            Some("1760000000"),
            black_box(body),
            &Bot,
            &env,
        )));
    });
    println!(
        "{:<34} {:>6} | {:>12} {:>8} {:>8} | {allocs:>12} {us:>8.2}",
        "handle_interaction, hex key",
        body.len(),
        "",
        "",
        ""
    );

    // As `process` runs it: the key parsed once per isolate
    let parsed = block_on(PublicKey::from_hex(&public_key)).unwrap();
    let us = time(2_000, || {
        black_box(block_on(handle_interaction_with_key(
            &parsed,
            Some(&signature),
            Some("1760000000"),
            black_box(body),
            &Bot,
            &env,
        )));
    });
    println!(
        "{:<34} {:>6} | {:>12} {:>8} {:>8} | {:>12} {us:>8.2}",
        "handle_interaction, cached key",
        body.len(),
        "",
        "",
        "",
        ""
    );
    println!(
        "\nsize_of: Interaction {} B, InteractionResponse {} B",
        std::mem::size_of::<Interaction>(),
        std::mem::size_of::<composure::types::InteractionResponse>()
    );
}
