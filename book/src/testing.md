# Testing

Bots built with Composure are tested in plain `cargo test`, without Discord or a platform: a
Command is a function, an App is a handler that takes a parsed Interaction, and
[`composure::testing`](https://docs.rs/composure/latest/composure/testing/index.html) stands in
for the rest. Turn it on for tests only:

```toml
[dev-dependencies]
composure = { version = "0.2", features = ["testing"] }
```

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

## Running an Interaction

[`testing::answer`](https://docs.rs/composure/latest/composure/testing/fn.answer.html) runs an App the
way an Adapter does: route the Interaction, call the handler, and answer an error the way the
App renders errors (an ephemeral message by default). Build the Interaction with the module's builders, and
read the response with [`ResponseExt`](https://docs.rs/composure/latest/composure/testing/trait.ResponseExt.html):

```rust
use composure::{
    app, command,
    testing::{self, ResponseExt, TestPlatform},
};

/// Repeat some text
#[command]
fn echo(#[option("What")] text: &str, #[option("Times")] times: Option<u8>) -> String {
    text.repeat(times.unwrap_or(1).into())
}

#[app(handlers(echo))]
pub struct Bot;

# fn main() {
let response = testing::answer(
    &Bot,
    testing::command("echo").option("text", "hi").option("times", 2),
    &TestPlatform::new(),
);
assert_eq!(response.text().as_deref(), Some("hihi"));
assert!(!response.is_ephemeral());

// `text` is required: the user gets an ephemeral error
let response = testing::answer(&Bot, testing::command("echo"), &TestPlatform::new());
assert!(response.is_ephemeral());
# }
```

[`testing::run`](https://docs.rs/composure/latest/composure/testing/fn.run.html) returns the
handler's `Result` instead, to check the error itself. Every builder starts from what Discord
sends: invoked by `testing::users::alice()` in a DM, unless told otherwise.

| Builder | |
|---|---|
| `command("name")` | `.option("text", "hi")`, `.option("times", 3)`, `.user_option("who", &user)`, `.option_json(..)` for channels and roles; `.subcommand("ban")`, `.group("mod")` |
| `user_command("Avatar", &user)`, `message_command("Quote", "text")` | Context menu Commands on a user or a message |
| `autocomplete("name").focused("option", "typed")` | The user typing in an Option |
| `component("custom_id")` | A button click; `.values(["red"])` for a select |
| `modal("custom_id")` | A submit: `.field("title", "Hi")`, `.select("kind", ["Bug"])`, `.input(json!(..))` for any other input |
| On every builder | `.by(&user)`, `.in_guild(id)`, `.permissions(..)`, `.locale("fr")`, `.channel(id)`, `.message(json!(..))`, `.json(\|json\| ..)` for anything else |

Build `custom_id`s with the handler's `custom_id(..)` function, so the test follows the
encoding: `testing::component(&counter::custom_id(4))`.

## A stand-in Platform

Commands taking `Ctx<impl Platform>` (REST, deferring) or `Ctx<impl HasData<..>>` (Bot Data) run
against [`TestPlatform`](https://docs.rs/composure/latest/composure/testing/struct.TestPlatform.html).
It records REST requests in a [`MockTransport`](https://docs.rs/composure/latest/composure/testing/struct.MockTransport.html)
(answering 200 `{}` unless told otherwise) and keeps Background Work until
`run_background()`, so a test checks the immediate response, runs the work, then checks what was
sent:

```rust
use composure::{
    app, command,
    command::{Ctx, Deferred, HandlerError, Platform},
    rest::{EditMessage, Method},
    testing::{self, ResponseExt, TestPlatform},
};

/// Count to three
#[command]
fn countdown(ctx: Ctx<impl Platform>) -> Deferred {
    ctx.defer(async |ctx| {
        for n in ["3...", "2...", "1..."] {
            let edit = EditMessage { content: Some(n), ..Default::default() };
            ctx.client().edit_original(&edit).await?;
        }
        Ok::<_, HandlerError>("Liftoff!")
    })
}

#[app(handlers(countdown))]
pub struct Bot;

# fn main() {
let platform = TestPlatform::new();
let response = testing::answer(&Bot, testing::command("countdown"), &platform);
assert_eq!(response.response_type(), 5); // deferred: "thinking..."
assert!(platform.transport().requests().is_empty());

platform.run_background().unwrap();
let transport = platform.transport();
assert_eq!(transport.sent().len(), 4);
assert_eq!(transport.sent()[0].0, Method::Patch);
assert_eq!(transport.body(3)["content"], "Liftoff!");
# }
```

- **Answers:** `MockTransport::new().respond(404, "..")` queues answers in order, and
  `.answering(|request| ..)` answers by route, for canned reads or other APIs the bot calls.
  Pass it in with `.with_transport(..)`.
- **Bot Data:** `.with_data(config)` sets it, and `.with_bot_token(None)` removes the token
  `ctx.bot()` uses.
- **Your own traits:** a bot whose handlers need more from their State (a key-value store, say)
  implements its trait on `TestPlatform<YourData>` in its own crate. An integration test can't:
  the orphan rule forbids it. There, wrap a `TestPlatform` in a struct of your own that forwards
  `Platform` and `HasData` to it and adds the rest.

## Signed requests

[`adapter::handle_interaction`](https://docs.rs/composure/latest/composure/adapter/fn.handle_interaction.html) and
[`handle_event`](https://docs.rs/composure/latest/composure/adapter/fn.handle_event.html) run the whole Adapter pipeline (signature check,
parsing, dispatch, error rendering) without a platform, and return the HTTP status and body an
Adapter would send. A [`TestKey`](https://docs.rs/composure/latest/composure/testing/struct.TestKey.html)
stands in for Discord's key:

```rust
use composure::{
    adapter::{InteractionOutcome, handle_interaction},
    app, command,
    testing::{self, TestKey, block_on},
};

/// Ping
#[command]
fn ping() -> &'static str { "Pong" }

#[app(handlers(ping))]
pub struct Bot;

# fn main() {
let key = TestKey::default();
let request = key.signed(testing::command("ping").body());

let outcome = block_on(handle_interaction(
    &key.public_key(), Some(&request.signature), Some(&request.timestamp), &request.body, &Bot, &(),
));
assert_eq!(outcome.status(), 200);

let forged = block_on(handle_interaction(
    &key.public_key(), Some(&request.signature), Some("1700000001"), &request.body, &Bot, &(),
));
assert!(matches!(forged, InteractionOutcome::Unauthorized));
# }
```

`request.headers()` gives both signature headers, to send to a running server.

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

The repository's tests go further:
- **`examples/bot/tests/`:** `offline.rs` runs every example Command on a `TestPlatform`, `regression.rs` replays signed fixtures for every Interaction shape, and `perf.rs` holds each path to an allocation budget.
- **`bots/composure/tests/offline.rs`:** wraps a `TestPlatform` to add the bot's own key-value store.
