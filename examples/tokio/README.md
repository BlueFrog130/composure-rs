# Composure example bot on a hosted server (tokio)

Serves the [example bot](../bot)'s shared commands from a plain process with [`composure_adapter_tokio`](../../crates/adapters/tokio) (an HTTP server on tokio, each Interaction on its own task), plus what only a server has. `src/lib.rs` declares its App with `#[app(handlers(Bot, file, uptime, primes), theme = THEME)] pub struct TokioBot;`; [`src/main.rs`](src/main.rs) serves it with a `Server` as the `Host` data.

[`src/server.rs`](src/server.rs): the server's disk, process and threads, through `Ctx<Host<Server>>`

| Command | Shows |
|---|---|
| `/file` | Reading the local filesystem with `tokio::fs`, sandboxed to `FILES_DIR` (default [`files/`](files)); `#[autocomplete]` listing the directory; text inline, long or binary files attached |
| `/uptime` | Process data handed to `with_data` (`Server::started`), the process id and the runtime's worker threads |
| `/primes` | A Deferred Response: `ctx.defer(..)` answers "thinking..." at once, then Background Work (a tokio task shutdown waits for) edits in progress, runs the sieve on `tokio::task::spawn_blocking`, and replaces the reply with a themed success embed |

## Run locally

```sh
cp examples/tokio/.env.example examples/tokio/.env   # fill in DISCORD_PUBLIC_KEY (and DISCORD_TOKEN for REST commands)
cargo run -p composure_example_tokio                 # http://0.0.0.0:8080 (PORT to change)
```

Discord needs a public HTTPS URL, so expose it with a tunnel (e.g. `cloudflared tunnel --url http://localhost:8080`) or a TLS-terminating reverse proxy, and set the application's **Interactions Endpoint URL** to it. Discord sends a signed Ping when you save; the bot must answer it for the save to succeed. For Webhook Events, set the Webhook Events URL to the same address plus `/events` and enable `Application Authorized`.

Logs go to stderr through `tracing_subscriber::fmt`, filtered by `RUST_LOG` (default `info,composure=debug`): every line logged while handling an Interaction carries its `interaction` span (id, type, name, guild, user).

`GET /health` answers 200 for load balancers. Ctrl-C or SIGTERM stops accepting and waits up to 30 seconds for in-flight requests and Background Work (`/countdown`, `/report`, ...).

## Configuration

| Variable | |
|---|---|
| `DISCORD_PUBLIC_KEY` | Required; the server refuses to start without a valid key |
| `DISCORD_TOKEN` | The bot token, for `/whoami`, `/announce`, `/dm-me`, `/report`, Thumbs up |
| `DISCORD_APPLICATION_ID` | Only for REST outside Interactions (`host.bot()`) |
| `GREETING` | Read by `/greet` |
| `FILES_DIR` | The only directory `/file` reads, `examples/tokio/files` by default |
| `COMPOSURE_SCOPE_DEV` | For the sync bin: guild ids `/mod` and `/announce` go to |
| `PORT` | Port to serve on, 8080 by default |

Variables already set in the environment win over `.env`.

## Sync

The `sync-tokio` bin publishes the shared commands plus `/file`, `/uptime`, `/primes`. It loads `.env` from this directory (copy `.env.example`; variables already set in the environment win):

```sh
cp .env.example .env   # fill in DISCORD_TOKEN, DISCORD_APPLICATION_ID, COMPOSURE_SCOPE_DEV (where /mod goes)
cargo run -p composure_example_tokio --bin sync-tokio -- --dry-run # show the plan
cargo run -p composure_example_tokio --bin sync-tokio              # apply after confirming
cargo run -p composure_example_tokio --bin sync-tokio -- --yes     # apply without asking (CI)
cargo run -p composure_example_tokio --bin sync-tokio -- --guild <guild id>  # everything to one guild, for testing
```

`--help` lists every flag; outside a terminal it only applies with `--yes`. See the [`composure_sync` README](../../crates/sync/README.md).

## Test

`cargo test -p composure_example_tokio` serves on a free port and replays the regression fixtures from `examples/bot/tests/common` over HTTP (every interaction shape must get the same status and JSON as through `composure::adapter` directly), then runs `/file` (including paths that try to leave the directory), its autocomplete and `/uptime` against `TokioBot`. `/primes` runs end to end against a stand-in Discord (`HttpTransport::with_api_base`): the reply is type 5, then the progress and result edits arrive as `PATCH .../messages/@original`.
