# Local development

Discord only talks to a public HTTPS URL. `cargo composure dev` is the whole loop: it serves the
bot, gives it a public URL with a tunnel, points your development application at that URL, and
keeps the bot and its Commands up to date on every save.

## Start from a template

[`cargo-generate`](https://github.com/cargo-generate/cargo-generate) creates a bot with a
`/hello` Command, a Sync binary, a test and the platform's configuration. It asks for the two ids
below; leave them blank to fill them in later.

```sh
cargo install cargo-generate
cargo generate --git https://github.com/BlueFrog130/composure-rs templates/cloudflare --name my-bot
cargo generate --git https://github.com/BlueFrog130/composure-rs templates/tokio --name my-bot
```

## A development application

Create a second application in the [Developer Portal](https://discord.com/developers/applications)
for development, so pointing it at your machine never touches the bot people use. Note its
**application id** and **public key** (General Information) and its **bot token** (Bot), then add
it to a test server through *Installation*.

## `cargo composure dev`

Install the tool and [cloudflared](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/),
which opens the tunnel:

```sh
cargo install cargo-composure
```

Name the application and the test server in the bot's `Cargo.toml`. Neither id is a secret, so
commit them and everyone on the project develops against the same application:

```toml
[package.metadata.composure.dev]
application = "1100146565504565411" # General Information > Application ID
guild = "798662131062931547"        # Developer Mode, then the server's context menu > Copy Server ID
```

Put the application's secrets in `.env` next to `Cargo.toml` (gitignored), then start it:

```sh
cp .env.example .env             # DISCORD_PUBLIC_KEY, DISCORD_TOKEN
cargo composure dev              # -p <package> from elsewhere in a workspace
```

Each stage logs one line as it finishes:

```txt
[composure] bot serving (6.1s)
[composure] dev Sync to guild 798662131062931547: 2 created
[composure] tunnel connected https://example-words-here.trycloudflare.com (6.2s)
[composure] https://example-words-here.trycloudflare.com answers through the tunnel (6.9s)
[composure] Dev Application 1100146565504565411 -> https://example-words-here.trycloudflare.com (8.1s)
```

Your bot's own logs and the compiler's output print in between, unchanged. Ctrl-C stops
everything. The endpoint keeps pointing at the old tunnel until the next run.

## What it does

1. **Builds and serves the bot.** On Workers (a package with a `wrangler.toml`) it runs your
   `[build] command` with `--release` swapped for `--dev`, then `wrangler dev`. On a server it runs
   `cargo build`, then your server bin with `PORT` set.
2. **Opens a tunnel**, a public HTTPS URL that forwards to the bot's port. A quick tunnel gets a
   new hostname each run and needs no account.
3. **Points the application at it.** Once the bot answers through the tunnel, it sets the
   **Interactions Endpoint URL**, and the **Webhook Events URL** (plus `/events`) if the application
   already has one. Discord sends a signed Ping before accepting a URL, which the Adapters answer.
4. **Syncs Commands to your test server**, with your Sync binary's `--guild`. Guild Commands
   update instantly. Commands the bot no longer has are deleted from that guild; global Commands
   are left alone.

On every save it rebuilds, swaps the new bot in, then syncs. It watches each local package the bot
is built from: their `src/`, `Cargo.toml` and `build.rs`, plus the bot's `.env`. A build that fails
prints its errors and the last good bot keeps serving. Swapping a server is a stop and a start:
Interactions in that moment fail, and Background Work in flight is dropped.

A stage that fails logs `<stage> failed: ...` with the last thing it saw, and the bot keeps
running. [Troubleshooting](troubleshooting.md#the-dev-loop) lists the messages.

## Settings

| `[package.metadata.composure.dev]` | | Unset |
|---|---|---|
| `application` | Your development application's id, or `--application`. The only application ever changed, and only when `DISCORD_TOKEN` is its token. | The tunnel opens and the URL to set by hand is printed. Nothing is written to Discord. |
| `guild` | The test server Commands go to, or `--guild`. | No Sync. |
| `tunnel` | `"quick"`, `{ url = "https://.." }` for a tunnel you already run, forwarding to `port`, or `false` for none. | `"quick"` |
| `port` | The local port. It must be free. | 8787 on Workers, 8080 on a server |
| `bin` | The server bin. | `default-run`, else the only bin that isn't the Sync bin |
| `sync_bin` | Your Sync binary. Without one, a warning and no Sync. | `sync` |
| `wrangler_env` | `wrangler dev --env`. | None |

Use a separate application for development. If `DISCORD_TOKEN` belongs to any other application,
a production token left in `.env` for instance, nothing is repointed or synced: a warning says so
and prints the URL to set by hand.

The tool finds cloudflared through `CLOUDFLARED_PATH`, then `PATH`.

## Without the tool

Each step also works by hand, for a platform the tool doesn't cover or a tunnel of your own.

Run the bot: `npx wrangler dev` (<http://localhost:8787>, secrets from `.env`) or `cargo run`
(<http://localhost:8080>). Give it a public URL:

```sh
cloudflared tunnel --url http://localhost:8787   # prints https://<random>.trycloudflare.com
ngrok http 8787                                  # or ngrok
```

Paste that URL into the application's **Interactions Endpoint URL** (General Information) and save.
Discord checks it before accepting: it sends a signed Ping, which must be answered with a Pong, and
requests with invalid signatures, which must be refused with 401. The Adapters do both, so a save
that fails means the bot isn't reachable at that URL, or its `DISCORD_PUBLIC_KEY` belongs to
another application. For [Webhook Events](events.md), set the **Webhook Events URL** to the same URL
plus `/events` and enable the events you handle.

Then send every Command to your test server, instead of the global Commands every server shares:

```sh
cargo run --bin sync -- --guild <test server id>
```

`--guild` ignores [Scopes](commands.md#scopes) and leaves global Commands alone. Run it again
whenever a Command's name, Options or description change.

## Logs and tests

`wrangler dev` prints what the Worker logs; the tokio template logs to stderr, filtered by
`RUST_LOG` (`RUST_LOG=info,composure=debug` shows every Interaction). Handler errors are logged
with their causes, and the user gets an ephemeral message instead of Discord's failure
([Troubleshooting](troubleshooting.md)).

Most changes don't need Discord at all: `cargo test` runs the Commands against stand-ins
([Testing](testing.md)).
