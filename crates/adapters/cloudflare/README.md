# composure_adapter_cloudflare

**Run your Discord bot on Cloudflare's edge.** Serve a [Composure](https://github.com/BlueFrog130/composure-rs)
bot from a Cloudflare Worker: no server to run, scaled to zero between requests, and for many
bots within the Workers free plan (100,000 requests a day).

[![Crates.io](https://img.shields.io/crates/v/composure_adapter_cloudflare.svg?logo=rust)](https://crates.io/crates/composure_adapter_cloudflare)
[![Docs.rs](https://img.shields.io/docsrs/composure_adapter_cloudflare?logo=docs.rs)](https://docs.rs/composure_adapter_cloudflare)
[![CI](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/composure_adapter_cloudflare.svg)](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.91+-orange.svg?logo=rust)](https://blog.rust-lang.org/)
[![Cloudflare Workers](https://img.shields.io/badge/Cloudflare-Workers-F38020?logo=cloudflareworkers&logoColor=white)](https://workers.cloudflare.com/)
[![WASM](https://img.shields.io/badge/target-wasm32-654FF0?logo=webassembly&logoColor=white)](https://webassembly.org/)

- **No crypto in your bundle**: signatures are checked with Workers' built-in Web Crypto, the key imported once per isolate
- **Small, quick to start**: the full example bot is about 307 KB of gzipped WASM; see [Cold starts](#cold-starts) for the build settings
- **Answer late**: `ctx.defer(..)` replies at once and finishes the work with `wait_until`
- **KV, D1, R2 and friends** through the `Worker` State, in any Command
- **Webhook Events** (app installs, entitlements) on `/events`, and a Bot Client for cron triggers
- **Workers Logs**: optional `tracing` support that turns every span field into a searchable log field

The adapter verifies the request signature against the `DISCORD_PUBLIC_KEY` secret, answers
Discord's Ping, and dispatches everything else to your handler, passing a `Worker` (the
worker's `Env`, `Context`, and your Bot Data) as State.

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

/// Read a var from wrangler.toml
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

## Configuration

The `Credentials` (public key, bot token, application id) come from the `DISCORD_PUBLIC_KEY`, `DISCORD_TOKEN` and `DISCORD_APPLICATION_ID` vars or secrets (`credentials(&env)`). Load them another way, other names or the Secrets Store, with `.with_credentials(async |env: Env| ..)`. Your own configuration is Bot Data: `.with_data(async |env: Env| ..)` builds it, Commands read it with `ctx.state().data()`, or `Ctx<impl HasData<Data = Config>>` to stay platform-agnostic. Both loaders run on an isolate's first request and are reused by the next ones; an error is logged, answered with 500, and retried. Bot Data is `Send + Sync`, so bindings (KV, D1) stay on the `Env`.

```rust,ignore
CloudflareInteractionBot::new(req, env, ctx)
    .with_handler(Bot)
    .with_data(async |env: Env| Ok(Config { greeting: env.var("GREETING")?.to_string() }))
    .process()
    .await
```

## State and REST

`Worker<T>` derefs to `Env` and implements `composure::command::Platform`, which gives Commands:

- `ctx.client()`: the Interaction Client (follow-ups, the original response); needs no secret.
- `ctx.bot()`: the Bot Client, from the Credentials' bot token; an error when there is none.
- `ctx.defer(async |ctx| ..)`: replies with a Deferred Response and runs the closure with `wait_until`; its output edits the original response. `Ephemeral(ctx.defer(..))` defers ephemerally. Background Work must finish within Workers' post-response limit (about 30 seconds).

Requests go through `FetchTransport` (Workers `fetch`). A response carrying files (`InteractionResponse::with_file`) is sent through the callback endpoint as multipart and the request is answered with 202.

## Webhook Events

Requests to `/events` (change with `.with_events_path(..)`) are Webhook Events: verified like Interactions, acknowledged with 204 at once, then handed to the handler's `EventHandler` as Background Work. Register one on the App with `#[app(.., events = MyEvents)]` and set the app's Webhook Events URL to `https://<worker>/events`. Unknown events reach `EventHandler::unknown` with their raw JSON. `handle_event(..)` runs the same steps without the Workers runtime for tests.

## Outside Interactions

`bot_client(&env)` builds a Bot Client from the `DISCORD_TOKEN` secret and the `DISCORD_APPLICATION_ID` var, for cron triggers, queues, or other events (with Credentials loaded another way, `credentials.bot_client(FetchTransport)`):

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

Handler errors are logged at ERROR, unknown commands and invalid Options at WARN, `HandlerError::user` at DEBUG, inside the span they happened in (with `error`, `error_kind` and, for errors with causes, `sources` fields), so the line carries the Interaction's or event's context. Without the feature the adapter writes the same messages straight to the console and nothing else is compiled in; with it the example bot grows by about 55 KB of wasm, its first request takes about 5 ms more and a warm one about 0.2 ms more at `DEBUG`; `init()` (INFO) skips Composure's per-Interaction DEBUG lines, which are most of that.

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

The example bot (`examples/cloudflare`) builds to about 927 KB of wasm (307 KB gzipped) with `tracing`. Parsing, dispatching and answering an interaction takes a few microseconds and 20-45 small allocations; `examples/bot/tests/perf.rs` measures each path and fails if one allocates more than its budget. `FetchTransport` keeps only the rate limit headers and `content-type` of REST responses (`composure::rest::DISCORD_HEADERS`).

## Cold starts

A new isolate fetches and validates the module, runs its top level and `#[event(start)]`, then serves the request. Workers compile each function on its first call, so that first request also compiles all the code it touches, and how much code that is depends on how the bundle was optimized. Two build settings, which the example uses, matter most:

```toml
# Cargo.toml: worker-build's wasm-opt step
[package.metadata.wasm-pack.profile.release]
wasm-opt = ["-Oz", "--one-caller-inline-max-function-size", "0"]
```

```toml
# .cargo/config.toml, next to Cargo.toml (worker-build runs cargo there)
[target.wasm32-unknown-unknown]
rustflags = [
    "-C", "link-arg=--strip-all",
    "-C", "link-arg=--keep-section=__wasm_bindgen_unstable",
    "-C", "link-arg=--keep-section=target_features",
]
```

- **No one-caller inlining.** binaryen inlines every function that has one caller, whatever its size, which merges a bot into a few huge functions (the example's fetch handler was 94 KB); the first request then compiles every Command in them. Kept apart, the first request compiles only the code it runs; warm requests got faster too.
- **No name section.** worker-build 0.8.7 keeps it for readable stack traces: 29% of the bundle, parsed on every cold start. `strip = true` in the profile also drops sections wasm-bindgen needs; these flags keep them. Panics then show `wasm-function[N]` frames. A worker-build that drops names for explicit `wasm-opt` arguments makes the flags redundant.

Composure does its part: dispatch polls each Interaction kind and each handler in a function of its own (`composure::adapter::outlined`), and this adapter does the same for Webhook Events and responses with files.

On workerd ([`bench/tests/coldstart.rs`](https://github.com/BlueFrog130/composure-rs/blob/main/crates/adapters/cloudflare/bench/tests/coldstart.rs), the example bot, each step minus the same step for a JS Worker answering alike, median of 25 fresh starts):

| Build | wasm | gzip | startup | first request | warm request |
|---|--:|--:|--:|--:|--:|
| 0.1.0 with worker-build's defaults | 1298 KB | 389 KB | 18.4 ms | 18.4 ms | 0.87 ms |
| the example's settings | 927 KB | 307 KB | 14.5 ms | 12.5 ms | 0.64 ms |
| a bare workers-rs Worker | 168 KB | 48 KB | 1.8 ms | 2.7 ms | 0.10 ms |

What else the benchmark found:

- `opt-level = "s"` or `3` instead of "z" makes the bundle 7% or 17% bigger for at most 0.1 ms on a warm request and none on the first: a request's time goes mostly to calls into JS (logging, Web Crypto, reading the request), not to Rust code.
- The `tracing` feature at `DEBUG` costs about 5 ms on the first request and 0.2 ms on every request; see [Logging](#logging).
- On nightly, std rebuilt with `-Cpanic=immediate-abort` (`-Zbuild-std=std,panic_abort`) drops panic messages and formatting: 6-7% less wasm and gzip, and panics become bare `unreachable` traps.
- Workers run startup while the TLS handshake is still in progress and route requests to warm isolates where they can ([Eliminating Cold Starts 2](https://blog.cloudflare.com/eliminating-cold-starts-2-shard-and-conquer/)), so most requests never see these costs; the startup limit is 1 second.

`bench/coldstart/size.mjs` shows where a build's bytes go (twiggy can't read modules with exception handling) and `profile.mjs` CPU-profiles the first requests.

## License

Licensed under the [Apache License, Version 2.0](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE).
