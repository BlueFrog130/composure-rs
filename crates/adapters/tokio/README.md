# composure_adapter_tokio

**Your Discord bot as a plain HTTP server.** Serve a [Composure](https://github.com/BlueFrog130/composure-rs)
bot from any machine, container or PaaS with tokio and hyper: every Interaction on its own task
across all cores, with the same answers as on Cloudflare Workers.

[![Crates.io](https://img.shields.io/crates/v/composure_adapter_tokio.svg?logo=rust)](https://crates.io/crates/composure_adapter_tokio)
[![Docs.rs](https://img.shields.io/docsrs/composure_adapter_tokio?logo=docs.rs)](https://docs.rs/composure_adapter_tokio)
[![CI](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/composure_adapter_tokio.svg)](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.91+-orange.svg?logo=rust)](https://blog.rust-lang.org/)
[![tokio](https://img.shields.io/badge/runs_on-tokio-463F3A?logo=rust&logoColor=white)](https://tokio.rs/)

- **Production-ready defaults**: `GET /health`, a 1 MiB body limit, panics isolated to their request, graceful shutdown on Ctrl-C / SIGTERM
- **Configured from the environment**: `DISCORD_PUBLIC_KEY`, `DISCORD_TOKEN`, `PORT`, and you're serving
- **Your data in every Command**: `.with_data(..)` hands it to handlers through `Host<T>`
- **Background Work you can trust**: deferred work is tracked and awaited on shutdown
- **Testable**: point the HTTP client at a stand-in Discord with `HttpTransport::with_api_base`

## Usage

```rust,ignore
use composure_adapter_tokio::TokioInteractionBot;

#[tokio::main]
async fn main() -> Result<(), composure_adapter_tokio::Error> {
    TokioInteractionBot::from_env()? // DISCORD_PUBLIC_KEY, DISCORD_TOKEN?, DISCORD_APPLICATION_ID?, PORT?
        .with_handler(Bot)            // an `#[app]`
        .serve()                      // 0.0.0.0:$PORT (8080), until Ctrl-C / SIGTERM
        .await
}
```

- Handlers get a `Host<T>` State: your data from `.with_data(..)` (also via `Deref`), plus the bot token, an `HttpTransport` (async `reqwest`, rustls), and Background Work as tokio tasks. `Ctx<impl Platform>` commands, `ctx.defer(..)` and `ctx.bot()` work as on Workers.
- Handler futures are `Send` (ADR 0012), so handlers can `tokio::spawn` work that owns a `DeferredCtx` or a clone of the `Host`.
- `HttpTransport::with_api_base(url)` sends what would go to Discord's API elsewhere: a proxy, or a stand-in Discord in tests.
- REST outside Interactions: `bot.host().bot()` (needs the token and `DISCORD_APPLICATION_ID`), and `host.spawn(..)` for tasks shutdown should wait for.
- Same answers as the Cloudflare Adapter (both go through `composure::adapter`): 401 on a bad signature, 400 on a bad body, ephemeral error messages for handler errors, files through the callback endpoint then 202, Webhook Events on `/events` ACKed with 204. On top: `GET /health` is 200, bodies over 1 MiB are 413, a panicking handler is 500.
- Shutdown stops accepting and waits up to 30s (`.with_grace(..)`) for in-flight requests and Background Work. `.with_shutdown(future)` replaces the Ctrl-C / SIGTERM signal.
- Plain HTTP only: Discord needs HTTPS, so run it behind a TLS-terminating proxy or tunnel.
- Logs through `tracing`; install a subscriber (e.g. `tracing_subscriber::fmt`) to see them, Composure's `interaction` / `background_work` / `event` / `rest` spans included.

See [`examples/tokio`](https://github.com/BlueFrog130/composure-rs/tree/main/examples/tokio) for the example bot served this way.

## License

Licensed under the [Apache License, Version 2.0](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE).
