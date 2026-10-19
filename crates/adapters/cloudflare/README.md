# Cloudflare Adapter

Runs a Composure bot on Cloudflare Workers. The adapter verifies the request signature against the `DISCORD_PUBLIC_KEY` secret, answers Discord's Ping, and dispatches everything else to your handler, passing a `Worker` (the worker's `Env` and `Context`) as State.

## Usage

Based on the `worker-rust` template (`npm create cloudflare@latest -- --template=cloudflare/workers-sdk/templates/experimental/worker-rust`). Enable `composure`'s `macros` feature (and `tier1` for REST), define commands with `#[command]`, list them in an `#[app]`, and hand it to the bot:

```rust,no_run
use std::time::Duration;

use composure::{Ctx, app, command, command::{Deferred, HandlerError}};
use composure_adapter_cloudflare::{CloudflareInteractionBot, Worker};
use worker::*;

/// Say hello
#[command]
fn hello(ctx: Ctx) -> String {
    // Server nickname in guilds; display name or username in DMs
    let name = match (ctx.member(), ctx.user()) {
        (Some(member), _) => member.display_name(),
        (None, Some(user)) => user.display_name(),
        (None, None) => "unknown user",
    };
    format!("Hello, {name}!")
}

/// Read a var from wrangler.jsonc
#[command]
async fn greeting(ctx: Ctx<Worker>) -> Result<String> {
    Ok(ctx.state().var("GREETING")?.to_string())
}

/// Answer now, finish later
#[command]
fn slow(ctx: Ctx<Worker>) -> Deferred {
    ctx.defer(async |ctx| {
        ctx.sleep(Duration::from_secs(2)).await;
        let me = ctx.bot()?.get_current_user().await?.model()?;
        Ok::<_, HandlerError>(format!("{} is done", me.username))
    })
}

#[app(handlers(hello, greeting, slow))]
pub struct Bot;

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> Result<Response> {
    CloudflareInteractionBot::new(req, env, ctx)
        .with_handler(Bot)
        .process()
        .await
}
# fn main() {}
```

`Bot.with_fallback(handler)` sends components, modals, autocomplete, and commands the App does not handle to any `InteractionHandler`. Implement `InteractionHandler<Worker>` directly instead of using an App for full control.

## State and REST

`Worker` derefs to `Env` and implements `composure::command::Platform`, which gives Commands:

- `ctx.client()`: the Interaction Client (follow-ups, the original response); needs no secret.
- `ctx.bot()`: the Bot Client, from a `DISCORD_TOKEN` secret; an error when it is not set.
- `ctx.defer(async |ctx| ..)`: replies with a Deferred Response and runs the closure with `wait_until`; its output edits the original response. `Ephemeral(ctx.defer(..))` defers ephemerally. Background Work must finish within Workers' post-response limit (about 30 seconds).

Requests go through `FetchTransport` (Workers `fetch`). A response carrying files (`InteractionResponse::with_file`) is sent through the callback endpoint as multipart and the request is answered with 202.

## Webhook Events

Requests to `/events` (change with `.with_events_path(..)`) are Webhook Events: verified like Interactions, acknowledged with 204 at once, then handed to the handler's `EventHandler` as Background Work. Register one on the App with `#[app(.., events = MyEvents)]` and set the app's Webhook Events URL to `https://<worker>/events`. Unknown events reach `EventHandler::unknown` with their raw JSON. `handle_event(..)` runs the same steps without the Workers runtime for tests.

## Outside Interactions

`bot_client(&env)` builds a Bot Client from the `DISCORD_TOKEN` secret and the `DISCORD_APPLICATION_ID` var, for cron triggers, queues, or other events:

```rust,ignore
#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let bot = composure_adapter_cloudflare::bot_client(&env).unwrap();
    bot.send_message(channel_id, "Good morning").await.unwrap();
}
```

`Ctx<Worker>` ties a Command (and the App listing it) to Workers. To run it in host tests, take `Ctx<impl Platform>` (or `Ctx<impl YourTrait>`) and implement it for a stand-in; see `examples/bot/tests/offline.rs`.

Handler futures must be `Send` (ADR 0012, so the same bot also runs on a multi-threaded server), although a Worker runs on one thread: `Worker` and `FetchTransport` wrap their JS handles in `worker::send::SendWrapper` / `SendFuture` for you. A handler that awaits a JS future itself (KV, D1, `worker::Fetch`) wraps it the same way: `SendFuture::new(kv.get("key").text()).await`, and holds JS values across `.await` in a `SendWrapper`. Handlers are still dispatched statically, so no `async_trait` or boxing.

Sync the App's Commands with Discord from a host-only bin; see `examples/bot/` for a complete bot with a `sync` bin, served by `examples/cloudflare/`.

## Logging

Turn on the `tracing` feature to log through [`tracing`](https://docs.rs/tracing) (it also enables `composure/tracing`) and install the Workers subscriber once per isolate:

```toml
composure_adapter_cloudflare = { version = "0.1.0", features = ["tracing"] }
```

```rust,ignore
#[event(start)]
fn start() {
    composure_adapter_cloudflare::tracing::init(); // or init_with_level(LevelFilter::DEBUG)
}
```

Every event becomes one `console.error`/`warn`/`log`/`debug` call with an object, so Workers Logs indexes its fields: `level`, `target`, `message`, `spans`, plus the fields of the enclosing spans. Composure's `interaction` span adds `interaction_id`, `interaction_type`, `name` (the command, or the `custom_id` prefix), `guild_id`, `channel_id` and `user_id` to everything logged while handling an Interaction, Background Work included; `event` adds `event_type`; `rest` adds `method` and `route` (the template, never the token-bearing URL). Log from handlers with `tracing::info!(..)` and the line carries all of that. `tracing::ConsoleLayer` composes with other `tracing-subscriber` layers.

Handler errors are logged at ERROR, unknown commands and invalid Options at WARN, `HandlerError::user` at DEBUG, inside the span they happened in (with `error` and `error_kind` fields), so the line carries the Interaction's or event's context. Without the feature the adapter writes the same messages straight to the console and nothing else is compiled in; with it the example bot grows by about 96 KB of wasm (about 36 KB of that is Composure's instrumentation, the rest the subscriber).

## Signature validation

Signatures are checked with Workers' built-in Web Crypto (Ed25519, the default `web-crypto` feature), so no crypto code is compiled into the worker. The public key is parsed and imported once per isolate and reused: about 61 µs per verify of a 1 KB body on workerd, against 84 µs importing it each time and 97-132 µs with dalek (`bench/`).

To compile `ed25519-dalek` in instead, turn default features off. Building the curve at `opt-level = 3` instead of a size-optimized "z" makes a verify about 22% faster for about 15 KB more bundle:

```toml
composure_adapter_cloudflare = { version = "0.1.0", default-features = false }

[profile.release.package.curve25519-dalek]
opt-level = 3

[profile.release.package.sha2]
opt-level = 3
```

## Testing a handler

`handle_interaction(public_key, signature, timestamp, body, &handler, &state)` runs the same verify, parse, and dispatch steps as `process` without the Workers runtime. It returns an `InteractionOutcome` with the HTTP status and response, so a handler can be tested with signed payloads in plain `cargo test`. Commands generic over State (`ctx: Ctx`) can be tested with `&()` as the state. `handle_interaction_with_key` / `handle_event_with_key` take a parsed `PublicKey` instead, as `process` does. These are re-exported from `composure::adapter`, which every Adapter runs, so answers match across platforms. See `examples/bot/tests/offline.rs`, and `examples/bot/tests/regression.rs` for signed fixtures covering every interaction shape.

## Size and speed

The example bot (`examples/cloudflare`) builds to about 1.25 MB of wasm (373 KB gzipped) with `tracing`, 1.16 MB (344 KB) without. Parsing, dispatching and answering an interaction takes a few microseconds and 20-45 small allocations; `examples/bot/tests/perf.rs` measures each path and fails if one allocates more than its budget. `FetchTransport` keeps only the rate limit headers and `content-type` of REST responses (`composure::rest::DISCORD_HEADERS`).
