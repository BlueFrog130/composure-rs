# Cloudflare Adapter

Runs a Composure bot on Cloudflare Workers. The adapter verifies the request signature against the `DISCORD_PUBLIC_KEY` secret, answers Discord's Ping, and dispatches everything else to your handler, passing a `Worker` (the worker's `Env` and `Context`) as State.

## Usage

Based on the `worker-rust` template (`npm create cloudflare@latest -- --template=cloudflare/workers-sdk/templates/experimental/worker-rust`). Enable `composure`'s `macros` feature (and `tier1` for REST), define commands with `#[command]`, and hand the Command Set to the bot:

```rust,no_run
use std::time::Duration;

use composure::{Ctx, command, command::{Deferred, HandlerError}, commands};
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

commands! {
    pub struct Bot = [hello, greeting, slow];
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> Result<Response> {
    CloudflareInteractionBot::new(req, env, ctx)
        .with_handler(Bot)
        .process()
        .await
}
# fn main() {}
```

`Bot.with_fallback(handler)` sends components, modals, autocomplete, and commands not in the set to any `InteractionHandler`. Implement `InteractionHandler<Worker>` directly instead of using a Command Set for full control.

## State and REST

`Worker` derefs to `Env` and implements `composure::command::Platform`, which gives Commands:

- `ctx.client()`: the Interaction Client (follow-ups, the original response); needs no secret.
- `ctx.bot()`: the Bot Client, from a `DISCORD_TOKEN` secret; an error when it is not set.
- `ctx.defer(async |ctx| ..)`: replies with a Deferred Response and runs the closure with `wait_until`; its output edits the original response. `Ephemeral(ctx.defer(..))` defers ephemerally. Background Work must finish within Workers' post-response limit (about 30 seconds).

Requests go through `FetchTransport` (Workers `fetch`). A response carrying files (`InteractionResponse::with_file`) is sent through the callback endpoint as multipart and the request is answered with 202.

## Webhook Events

Requests to `/events` (change with `.with_events_path(..)`) are Webhook Events: verified like Interactions, acknowledged with 204 at once, then handed to the handler's `EventHandler` as Background Work. Register one on the Command Set with `commands! { ...; events = MyEvents; }` and set the app's Webhook Events URL to `https://<worker>/events`. Unknown events reach `EventHandler::unknown` with their raw JSON. `handle_event(..)` runs the same steps without the Workers runtime for tests.

## Outside Interactions

`bot_client(&env)` builds a Bot Client from the `DISCORD_TOKEN` secret and the `DISCORD_APPLICATION_ID` var, for cron triggers, queues, or other events:

```rust,ignore
#[event(scheduled)]
async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let bot = composure_adapter_cloudflare::bot_client(&env).unwrap();
    bot.send_message(channel_id, "Good morning").await.unwrap();
}
```

`Ctx<Worker>` ties a Command (and its Command Set) to Workers. To run it in host tests, take `Ctx<impl Platform>` (or `Ctx<impl YourTrait>`) and implement it for a stand-in; see `examples/cloudflare/tests/offline.rs`.

Workers futures are not `Send`; handlers are dispatched statically, so no `Send` bounds or `async_trait` are needed.

Sync the Command Set with Discord from a host-only bin; see `examples/cloudflare/` for a complete bot with a `sync` bin.

## Signature validation

By default signatures are checked with `ed25519-dalek` compiled into the worker. Enable the `web-crypto` feature to use Workers' built-in Web Crypto (Ed25519) instead: faster, and dalek drops out of the bundle.

```toml
composure_adapter_cloudflare = { version = "0.1.0", features = ["web-crypto"] }
```

## Testing a handler

`handle_interaction(public_key, signature, timestamp, body, &handler, &state)` runs the same verify, parse, and dispatch steps as `process` without the Workers runtime. It returns an `InteractionOutcome` with the HTTP status and response, so a handler can be tested with signed payloads in plain `cargo test`. Commands generic over State (`ctx: Ctx`) can be tested with `&()` as the state. See `examples/cloudflare/tests/offline.rs`, and `examples/cloudflare/tests/regression.rs` for signed fixtures covering every interaction shape.

## Todo

- [ ] Make package size smaller (simple build results in ~800 kb worker size)
