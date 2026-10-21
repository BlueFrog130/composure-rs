# Testing

Bots built with Composure are tested in plain `cargo test`, without Discord or a platform: a
Command is a function, an App is a handler that takes a parsed Interaction, and an Adapter's
State can be replaced by a stand-in.

## Calling the function

`#[command]` and the handler macros keep the function as written, so its logic is a unit test
away:

```rust
use composure::command;

/// Repeat some text
#[command]
fn echo(#[option("What")] text: &str, #[option("Times")] times: Option<u8>) -> String {
    text.repeat(times.unwrap_or(1).into())
}

# fn main() {
assert_eq!(echo("ab", Some(2)), "abab");
# }
```

## Dispatching an Interaction

[`dispatch`](https://docs.rs/composure/latest/composure/command/fn.dispatch.html) runs an App the way an Adapter does: route the
Interaction, call the handler, apply the Theme. Build the Interaction from the JSON Discord would
send, and pass any State the App accepts (`&()` for Apps whose Commands take `Ctx`):

```rust
use composure::{app, command, command::dispatch, types::Interaction};
use serde_json::json;

/// Repeat some text
#[command]
fn echo(#[option("What")] text: &str) -> String {
    text.to_uppercase()
}

#[app(handlers(echo))]
pub struct Bot;

# fn main() {
let interaction: Interaction = serde_json::from_value(json!({
    "id": "2", "application_id": "1", "type": 2, "token": "token", "version": 1,
    "user": { "id": "3", "username": "ada", "discriminator": "0", "global_name": null, "avatar": null },
    "data": { "id": "4", "name": "echo", "type": 1,
              "options": [{ "name": "text", "type": 3, "value": "hi" }] },
}))
.unwrap();

let response = futures::executor::block_on(dispatch(interaction, &Bot, &())).unwrap();
let json = serde_json::to_value(&response).unwrap();
assert_eq!(json["data"]["content"], "HI");
# }
```

Components and modal submits work the same way, with their `custom_id` in `data` (build it with
the handler's `custom_id(..)` function so the test follows the encoding). Any executor runs the
future; handlers do not depend on a runtime.

## A stand-in Platform

Commands taking `Ctx<impl Platform>` (or `Ctx<impl YourTrait>`) run against any State
implementing it. A test State records the REST requests through its
[`Transport`](https://docs.rs/composure/latest/composure/rest/trait.Transport.html) and keeps Background Work to run when the test says so:

```rust
use std::{sync::{Arc, Mutex}, time::Duration};
use composure::{
    command::{BackgroundWork, Platform},
    rest::{RawResponse, Request, Transport, TransportError},
};

/// Records requests; answers every one with `{}`
#[derive(Default)]
struct Discord { requests: Mutex<Vec<Request>> }

impl Transport for Discord {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        self.requests.lock().unwrap().push(request.clone());
        Ok(RawResponse { status: 200, headers: Vec::new(), body: b"{}".to_vec() })
    }

    async fn sleep(&self, _duration: Duration) {}
}

#[derive(Clone, Default)]
struct TestPlatform {
    discord: Arc<Discord>,
    work: Arc<Mutex<Vec<BackgroundWork>>>,
}

impl Platform for TestPlatform {
    type Transport = Arc<Discord>;

    fn transport(&self) -> Arc<Discord> {
        self.discord.clone()
    }

    fn spawn(&self, work: BackgroundWork) {
        self.work.lock().unwrap().push(work); // run later with block_on
    }

    fn bot_token(&self) -> Option<&str> {
        Some("test-token")
    }
}
# fn main() {}
```

Dispatch with `&TestPlatform::default()`, check the immediate response (a deferral, say), run
the queued [`BackgroundWork`](https://docs.rs/composure/latest/composure/command/type.BackgroundWork.html), then assert on the recorded
requests: the edit of the original response, the follow-ups, the bot calls.

## Signed requests

[`adapter::handle_interaction`](https://docs.rs/composure/latest/composure/adapter/fn.handle_interaction.html) and
[`handle_event`](https://docs.rs/composure/latest/composure/adapter/fn.handle_event.html) run the whole Adapter pipeline (signature check,
parsing, dispatch, error rendering) without a platform, and return the HTTP status and body an
Adapter would send. Sign test payloads with a key of your own:

```rust
use composure::{adapter::{InteractionOutcome, handle_interaction}, app, command};
use ed25519_dalek::{Signer, SigningKey};

/// Ping
#[command]
fn ping() -> &'static str { "Pong" }

#[app(handlers(ping))]
pub struct Bot;

# fn main() {
let key = SigningKey::from_bytes(&[7; 32]);
let public_key = hex::encode(key.verifying_key().to_bytes());

let body = br#"{"id":"2","application_id":"1","type":1,"token":"t","version":1}"#;
let timestamp = "1700000000";
let signature = hex::encode(key.sign(&[timestamp.as_bytes(), body].concat()).to_bytes());

let outcome = futures::executor::block_on(handle_interaction(
    &public_key, Some(&signature), Some(timestamp), body, &Bot, &(),
));
assert_eq!(outcome.status(), 200); // a Ping, answered with a Pong

let forged = futures::executor::block_on(handle_interaction(
    &public_key, Some(&signature), Some("1700000001"), body, &Bot, &(),
));
assert!(matches!(forged, InteractionOutcome::Unauthorized));
# }
```

## Definitions

What Sync would register is available without Discord too:
[`CommandSetDefinition::definitions`](https://docs.rs/composure/latest/composure/command/trait.CommandSetDefinition.html#tymethod.definitions) on an
App lists every Command Definition with its Scope, ready to compare against a snapshot.

```rust
use composure::{app, command, command::CommandSetDefinition};

/// Ping
#[command(scope = "dev")]
fn ping() -> &'static str { "Pong" }

#[app(handlers(ping))]
pub struct Bot;

# fn main() {
let definitions = Bot::definitions();
assert_eq!(definitions[0].scope, Some("dev"));
# }
```

The repository's `examples/bot/tests/` go further: `offline.rs` runs every example Command
against a stand-in Platform, `regression.rs` replays signed fixtures for every Interaction
shape, and `perf.rs` holds each path to an allocation budget.
