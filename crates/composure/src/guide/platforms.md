An Adapter serves an App on one platform. Everything platform-specific (reading requests, the
HTTP client, running Background Work, secrets) lives in the Adapter and reaches handlers as its
**State**: the `S` in [`Ctx<S>`](crate::command::Ctx).

| Adapter | State | Background Work | REST transport |
|---|---|---|---|
| [`composure_adapter_cloudflare`](https://docs.rs/composure_adapter_cloudflare) | `Worker`: the Worker's `Env` (vars, secrets, KV, D1, ...) and `Context` | `ctx.wait_until`, about 30 s after the response | `fetch` |
| [`composure_adapter_tokio`](https://docs.rs/composure_adapter_tokio) | `Host<T>`: your data `T` (from `.with_data(..)`, also through `Deref`), the bot token, the HTTP client | a `tokio::spawn`ed task, awaited on shutdown | async `reqwest` |

Both implement [`Platform`](crate::command::Platform), and answer every request the same way
because both run the shared pipeline in [`adapter`](crate::adapter): `401` for a bad signature,
`400` for a body that is not an Interaction, the handler's response (or its ephemeral error
message) as JSON, and responses with files through the callback endpoint then `202`.

# Platform-agnostic Commands

A Command's `ctx` type decides which States can serve it, and an App can only be served with a
State every Command it lists accepts. Keep Commands as general as they can be:

- `ctx: Ctx` (or no `ctx`) when they only answer
- `ctx: Ctx<impl Platform>` when they use REST or defer
- `ctx: Ctx<impl YourTrait>` when they need something from the platform

The last one is how a Command reads configuration without depending on an Adapter: declare a
trait for what it needs, implement it for each Adapter's State, and for a stand-in in tests.

```
use composure::{command, command::{Ctx, HandlerError}};

/// What `/greet` needs from the State
pub trait Vars {
    fn var(&self, name: &str) -> Option<String>;
}

/// Greet with the GREETING var
#[command]
fn greet(ctx: Ctx<impl Vars>) -> Result<String, HandlerError> {
    let greeting = ctx.state().var("GREETING").ok_or_else(|| HandlerError::new("GREETING is not set"))?;
    Ok(format!("{greeting} from Composure!"))
}

// In the bot, per Adapter:
//
// impl Vars for composure_adapter_cloudflare::Worker {
//     fn var(&self, name: &str) -> Option<String> {
//         self.env().var(name).ok().map(|var| var.to_string())
//     }
// }
//
// impl<T> Vars for composure_adapter_tokio::Host<T> {
//     fn var(&self, name: &str) -> Option<String> {
//         std::env::var(name).ok()
//     }
// }
# fn main() {}
```

A bot served on several platforms keeps its shared Commands in one App (or crate) generic like
this, and each platform's binary lists that App in its own, next to Commands that only work
there (`Ctx<Worker>` reading Workers KV, `Ctx<Host<MyData>>` reading local files). The
repository's `examples/` are built that way.

# Platform-specific Commands

When a Command is inherently tied to a platform, take its State directly:

```rust,ignore
use composure::{command, command::{Ctx, HandlerError}};
use composure_adapter_tokio::Host;

pub struct Server { started: std::time::Instant }

/// How long the server has been up
#[command]
fn uptime(ctx: Ctx<Host<Server>>) -> String {
    format!("Up for {:?}", ctx.state().started.elapsed()) // Host<T> derefs to T
}
```

# `Send` futures

Handler futures must be `Send` ([ADR 0012](https://github.com/BlueFrog130/composure-rs/blob/main/docs/adr/0012-send-futures.md)),
so the tokio Adapter can run each Interaction on any thread. Write `async fn` handlers normally;
the compiler points out a non-`Send` value (an `Rc`, a `RefCell` borrow) held across `.await`.

Workers run on one thread, but their JS handles are not `Send`: the Adapter's `Worker` and
transport wrap them for you, and a handler awaiting a JS future itself (KV, D1, `fetch`) wraps
it in `worker::send::SendFuture::new(..)`, or marks a helper `#[worker::send]`.

# Configuration

| Variable | Cloudflare | tokio | |
|---|---|---|---|
| `DISCORD_PUBLIC_KEY` | secret | env | Required: verifies requests |
| `DISCORD_TOKEN` | secret | env | The bot token, for [`ctx.bot()`](crate::command::Ctx::bot) |
| `DISCORD_APPLICATION_ID` | var | env | For a bot client outside Interactions (cron, startup) |
| `PORT` | | env | Port to serve on (8080) |

Webhook Events are served on `/events` (change it with `.with_events_path(..)`), Interactions
on every other path. The tokio Adapter also answers `GET /health`.

# Logging

With the `tracing` feature, Composure reports through [`tracing`](https://docs.rs/tracing). It
never installs a subscriber; the Adapter or your `main` does
(`composure_adapter_cloudflare::tracing::init()` on Workers, which writes to Workers Logs;
`tracing_subscriber::fmt().init()` or any other on a server).

| Span | Fields | Around |
|---|---|---|
| `interaction` | `interaction_id`, `interaction_type`, `name` (the Command, or the `custom_id` prefix), `guild_id`, `channel_id`, `user_id` | dispatching one Interaction |
| `background_work` | | deferred work, inside its `interaction` span |
| `event` | `event_type` | handling a Webhook Event |
| `rest` | `method`, `route` (the template, never the URL, which can hold a token) | one REST call |

Everything logged inside a handler, Background Work included, carries its Interaction's fields:
`tracing::info!(..)` in a Command is enough. Handler errors are logged in their span at ERROR,
unknown Commands and invalid Options at WARN, [`HandlerError::user`](crate::command::HandlerError::user)
errors at DEBUG.

# Writing an Adapter

A new platform needs little code: [`adapter`](crate::adapter) does the work every Adapter
shares, so an Adapter only maps HTTP to it.

1. Implement [`Platform`](crate::command::Platform) on the State: a
   [`Transport`](crate::rest::Transport) (`send` a [`Request`](crate::rest::Request), `sleep`),
   a way to [`spawn`](crate::command::Platform::spawn) Background Work after the response, and
   the bot token.
2. Parse the public key once into a [`PublicKey`](crate::auth::PublicKey) and keep it.
3. For each request, call
   [`handle_interaction_with_key`](crate::adapter::handle_interaction_with_key) (or
   [`handle_event_with_key`](crate::adapter::handle_event_with_key) on the events path) with the
   two signature headers and the body, and turn the
   [`InteractionOutcome`](crate::adapter::InteractionOutcome) into a response:
   [`status()`](crate::adapter::InteractionOutcome::status) and
   [`response()`](crate::adapter::InteractionOutcome::response) as JSON. When the response has
   [files](crate::types::InteractionResponse::files), send it with
   [`send_with_files`](crate::adapter::send_with_files) and answer `202` instead.
4. Render handler errors with the handler's
   [`theme()`](crate::command::InteractionHandler::theme), as the outcome already does.

The two existing Adapters (`crates/adapters/` in the repository) are a few hundred lines each.
