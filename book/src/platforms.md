# Platforms

An Adapter serves an App on one platform. Everything platform-specific (reading requests, the
HTTP client, running Background Work, secrets) lives in the Adapter and reaches handlers as its
**State**: the `S` in [`Ctx<S>`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html).

| Adapter | State | Background Work | REST transport |
|---|---|---|---|
| [`composure_adapter_cloudflare`](https://docs.rs/composure_adapter_cloudflare) | `Worker<T>`: the Worker's `Env` (vars, secrets, KV, D1, ..., also through `Deref`), its `Context`, and your Bot Data `T` (from `.with_data(..)`) | `ctx.wait_until`, about 30 s after the response | `fetch` |
| [`composure_adapter_tokio`](https://docs.rs/composure_adapter_tokio) | `Host<T>`: your Bot Data `T` (from `.with_data(..)`, also through `Deref`), the bot token, the HTTP client | a `tokio::spawn`ed task, awaited on shutdown | async `reqwest` |

Both implement [`Platform`](https://docs.rs/composure/latest/composure/command/trait.Platform.html) and [`HasData`](https://docs.rs/composure/latest/composure/command/trait.HasData.html), and answer every request the same way
because both run the shared pipeline in [`adapter`](https://docs.rs/composure/latest/composure/adapter/): `401` for a bad signature,
`400` for a body that is not an Interaction, the handler's response (or its ephemeral error
message) as JSON, and responses with files through the callback endpoint then `202`.

## Platform-agnostic Commands

A Command's `ctx` type decides which States can serve it, and an App can only be served with a
State every Command it lists accepts. Keep Commands as general as they can be:

- `ctx: Ctx` (or no `ctx`) when they only answer
- `ctx: Ctx<impl Platform>` when they use REST or defer
- `ctx: Ctx<impl HasData<Data = Config>>` when they read the bot's own configuration
- `ctx: Ctx<impl YourTrait>` when they need something else from the platform

## Bot Data

The bot's own configuration and resources (settings, a database pool, ...) are its **Bot
Data**: one typed value, built once and shared by every Interaction. Each Adapter takes it with
`.with_data(..)` and hands it to Commands through the State's
[`HasData`](https://docs.rs/composure/latest/composure/command/trait.HasData.html), so a Command reads it the same way on every platform, and
the bot loads it however that platform likes: `config`, `figment`, `envy` or `std::env` on a
server, the Worker's vars, secrets or KV on Workers.

```rust
use composure::{command, command::{Ctx, HasData}};

/// The bot's settings
pub struct Config {
    pub greeting: String,
}

/// Greet with the configured greeting
#[command]
fn greet(ctx: Ctx<impl HasData<Data = Config>>) -> String {
    format!("{} from Composure!", ctx.state().data().greeting)
}

// Per Adapter:
//
// // tokio: load it any way, e.g. `settings.get::<Config>("bot")?` with `config`
// TokioInteractionBot::from_env()?.with_handler(Bot).with_data(config)
//
// // Workers: built from the Env on an isolate's first request, then reused
// CloudflareInteractionBot::new(req, env, ctx)
//     .with_handler(Bot)
//     .with_data(async |env: Env| Ok(Config { greeting: env.var("GREETING")?.to_string() }))
# fn main() {}
```

When one platform's Bot Data holds more than the shared Commands need (a server's own state),
bound on a part of it, `Ctx<impl HasData<Data: AsRef<Config>>>`, and implement `AsRef<Config>`
for each Bot Data type (`Config` itself included). In tests, implement `HasData` on the stand-in
State. Bot Data must be `Send + Sync`; on Workers, bindings (KV, D1, ...) stay on the `Env`.

A bot served on several platforms keeps its shared Commands in one App (or crate) generic like
this, and each platform's binary lists that App in its own, next to Commands that only work
there (`Ctx<Worker>` reading Workers KV, `Ctx<Host<MyData>>` reading local files). The
repository's `examples/` are built that way.

## Platform-specific Commands

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

## `Send` futures

Handler futures must be `Send` ([ADR 0012](https://github.com/BlueFrog130/composure-rs/blob/main/docs/adr/0012-send-futures.md)),
so the tokio Adapter can run each Interaction on any thread. Write `async fn` handlers normally;
the compiler points out a non-`Send` value (an `Rc`, a `RefCell` borrow) held across `.await`.

Workers run on one thread, but their JS handles are not `Send`: the Adapter's `Worker` and
transport wrap them for you, and a handler awaiting a JS future itself (KV, D1, `fetch`) wraps
it in `worker::send::SendFuture::new(..)`, or marks a helper `#[worker::send]`.

## Credentials

What Composure itself needs is the application's [`Credentials`](https://docs.rs/composure/latest/composure/credentials/struct.Credentials.html): the
public key, and optionally the bot token and application id. Composure never reads them from
anywhere itself: build them in code, from the conventional names with
[`Credentials::from_lookup`](https://docs.rs/composure/latest/composure/credentials/struct.Credentials.html#method.from_lookup), or deserialize them with any
serde loader (fields `public_key`, `bot_token` or `token`, `application_id`).

| Variable | Cloudflare | tokio | |
|---|---|---|---|
| `DISCORD_PUBLIC_KEY` | secret | env | Required: verifies requests |
| `DISCORD_TOKEN` | secret | env | The bot token, for [`ctx.bot()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.bot) |
| `DISCORD_APPLICATION_ID` | var | env | For a bot client outside Interactions (cron, startup) |
| `PORT` | | env | Port to serve on (8080) |

Those are the defaults: `TokioInteractionBot::from_env()` and `CloudflareInteractionBot` read
them. Otherwise, pass `Credentials` loaded another way:

```rust,ignore
// tokio, with the `config` crate: a `[discord]` table in bot.toml, or DISCORD__PUBLIC_KEY, ...
let settings = config::Config::builder()
    .add_source(config::File::with_name("bot"))
    .add_source(config::Environment::default().separator("__"))
    .build()?;
TokioInteractionBot::new(settings.get::<Credentials>("discord")?)?

// Workers: other names, the Secrets Store, ...; loaded on an isolate's first request
CloudflareInteractionBot::new(req, env, ctx).with_credentials(async |env: Env| {
    let token = env.secret_store("BOT_TOKEN")?.get().await?.unwrap_or_default();
    Ok(Credentials::new(&env.var("PUBLIC_KEY")?.to_string())?.with_bot_token(token))
})
```

`Debug` on `Credentials` never shows the token.

Webhook Events are served on `/events` (change it with `.with_events_path(..)`), Interactions
on every other path. The tokio Adapter also answers `GET /health`.

## Logging

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
unknown Commands and invalid Options at WARN, [`HandlerError::user`](https://docs.rs/composure/latest/composure/command/struct.HandlerError.html#method.user)
errors at DEBUG.

## Writing an Adapter

A new platform needs little code: [`adapter`](https://docs.rs/composure/latest/composure/adapter/) does the work every Adapter
shares, so an Adapter only maps HTTP to it.

1. Take [`Credentials`](https://docs.rs/composure/latest/composure/credentials/struct.Credentials.html), and offer a loader for the platform's usual
   source on top (by the conventional names through
   [`Credentials::from_lookup`](https://docs.rs/composure/latest/composure/credentials/struct.Credentials.html#method.from_lookup)).
2. Implement [`Platform`](https://docs.rs/composure/latest/composure/command/trait.Platform.html) on the State: a
   [`Transport`](https://docs.rs/composure/latest/composure/rest/trait.Transport.html) (`send` a [`Request`](https://docs.rs/composure/latest/composure/rest/struct.Request.html), `sleep`),
   a way to [`spawn`](https://docs.rs/composure/latest/composure/command/trait.Platform.html#tymethod.spawn) Background Work after the response, and
   the bot token; and [`HasData`](https://docs.rs/composure/latest/composure/command/trait.HasData.html) for the Bot Data.
3. Parse the public key once into a [`PublicKey`](https://docs.rs/composure/latest/composure/auth/struct.PublicKey.html) and keep it.
4. For each request, call
   [`handle_interaction_with_key`](https://docs.rs/composure/latest/composure/adapter/fn.handle_interaction_with_key.html) (or
   [`handle_event_with_key`](https://docs.rs/composure/latest/composure/adapter/fn.handle_event_with_key.html) on the events path) with the
   two signature headers and the body, and turn the
   [`InteractionOutcome`](https://docs.rs/composure/latest/composure/adapter/enum.InteractionOutcome.html) into a response:
   [`status()`](https://docs.rs/composure/latest/composure/adapter/enum.InteractionOutcome.html#method.status) and
   [`response()`](https://docs.rs/composure/latest/composure/adapter/enum.InteractionOutcome.html#method.response) as JSON. When the response has
   [files](https://docs.rs/composure/latest/composure/types/enum.InteractionResponse.html#method.files), send it with
   [`send_with_files`](https://docs.rs/composure/latest/composure/adapter/fn.send_with_files.html) and answer `202` instead.
5. Render handler errors with the handler's
   [`theme()`](https://docs.rs/composure/latest/composure/command/trait.InteractionHandler.html#method.theme), as the outcome already does.

The two existing Adapters (`crates/adapters/` in the repository) are a few hundred lines each.
