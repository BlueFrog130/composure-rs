# composure_adapter_tokio

Serve a [Composure](../../../README.md) bot from a hosted server: an HTTP server on tokio (hyper) answering Discord's signed Interactions and Webhook Events, each Interaction on its own task across the runtime's threads.

```rust,ignore
use composure_adapter_tokio::TokioInteractionBot;

#[tokio::main]
async fn main() -> Result<(), composure_adapter_tokio::Error> {
    TokioInteractionBot::from_env()? // DISCORD_PUBLIC_KEY, DISCORD_TOKEN?, DISCORD_APPLICATION_ID?, PORT?
        .with_handler(Bot)            // a Command Set from `commands!`
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

See [`examples/tokio`](../../../examples/tokio) for the example bot served this way.
