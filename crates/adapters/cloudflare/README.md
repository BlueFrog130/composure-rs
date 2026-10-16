# Cloudflare Adapter

Runs a Composure bot on Cloudflare Workers. The adapter verifies the request signature against the `DISCORD_PUBLIC_KEY` secret, answers Discord's Ping, and dispatches everything else to your handler, passing the worker's `Env` as State.

## Usage

Based on the `worker-rust` template (`npm create cloudflare@latest -- --template=cloudflare/workers-sdk/templates/experimental/worker-rust`). Enable `composure`'s `macros` feature, define commands with `#[command]`, and hand the Command Set to the bot:

```rust,no_run
use composure::{Ctx, command, commands};
use composure_adapter_cloudflare::CloudflareInteractionBot;
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
async fn greeting(ctx: Ctx<Env>) -> Result<String> {
    Ok(ctx.state().var("GREETING")?.to_string())
}

commands! {
    pub struct Bot = [hello, greeting];
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    CloudflareInteractionBot::new(req, env)
        .with_handler(Bot)
        .process()
        .await
}
# fn main() {}
```

`Bot.with_fallback(handler)` sends components, modals, autocomplete, and commands not in the set to any `InteractionHandler`. Implement `InteractionHandler<Env>` directly instead of using a Command Set for full control.

`Ctx<Env>` ties a Command (and its Command Set) to Workers. To run it in host tests, take `Ctx<impl YourTrait>` and implement the trait for `Env` and a stand-in; see `/greet` in `examples/cloudflare/`.

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
- [ ] Add Discord REST integration (configurable to minimize package size)
