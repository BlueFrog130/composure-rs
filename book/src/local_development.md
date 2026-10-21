# Local development

Discord only talks to a public HTTPS URL. To develop a bot, run it on your machine, give it a
public URL with a tunnel, point a development application at that URL, and register the
Commands in a test server, where changes show up at once.

## Start from a template

[`cargo-generate`](https://github.com/cargo-generate/cargo-generate) creates a bot with a
`/hello` Command, a Sync binary, a test and the platform's configuration:

```sh
cargo install cargo-generate
cargo generate --git https://github.com/BlueFrog130/composure-rs templates/cloudflare --name my-bot
cargo generate --git https://github.com/BlueFrog130/composure-rs templates/tokio --name my-bot
```

## A development application

Create a second application in the [Developer Portal](https://discord.com/developers/applications)
for development, so pointing it at your machine never touches the bot people use. Note its
**public key** (General Information), its **application id**, and its **bot token** (Bot), then
add it to a test server through *Installation*.

## Running the bot

On Workers, `wrangler dev` serves the Worker locally and reads its secrets from `.dev.vars`:

```sh
cp .dev.vars.example .dev.vars   # DISCORD_PUBLIC_KEY, DISCORD_TOKEN
npx wrangler dev                 # http://localhost:8787
```

On a server, run it as is; the template reads `.env`:

```sh
cp .env.example .env             # DISCORD_PUBLIC_KEY, DISCORD_TOKEN, DISCORD_APPLICATION_ID
cargo run                        # http://localhost:8080
```

## A tunnel

A tunnel gives the local port a public HTTPS URL:

```sh
cloudflared tunnel --url http://localhost:8787   # prints https://<random>.trycloudflare.com
ngrok http 8787                                  # or ngrok
```

Use port 8080 for the tokio bot. Quick tunnels get a new URL each time they start; a named
tunnel (or a reserved ngrok domain) keeps one.

## The Interactions Endpoint URL

Paste the tunnel's URL into the development application's **Interactions Endpoint URL**
(General Information) and save. Discord checks it before accepting: it sends a signed Ping, which
must be answered with a Pong, and requests with invalid signatures, which must be refused with
401. The Adapters do both, so a save that fails means the bot isn't reachable at that URL, or
its `DISCORD_PUBLIC_KEY` belongs to another application.

For [Webhook Events](events.md), set the **Webhook Events URL** to the same URL plus `/events`
and enable the events you handle.

## Commands in a test server

Global Commands are shared by every server; while developing, send them all to your test server
instead, where Discord shows changes immediately:

```sh
cp .env.example .env             # DISCORD_TOKEN, DISCORD_APPLICATION_ID (Workers template)
cargo run --bin sync -- --guild <test server id>
```

`--guild` ignores [Scopes](commands.md#scopes) and leaves global Commands alone. Copy a
server's id with Developer Mode on (User Settings › Advanced), from the server's context menu.
Run it again whenever a Command's name, Options or description change; handler code changes
only need the bot restarted.

## Logs and tests

`wrangler dev` prints what the Worker logs; the tokio template logs to stderr, filtered by
`RUST_LOG` (`RUST_LOG=info,composure=debug` shows every Interaction). Handler errors are logged
with their causes, and the user gets an ephemeral message instead of Discord's failure
([Troubleshooting](troubleshooting.md)).

Most changes don't need Discord at all: `cargo test` runs the Commands against stand-ins
([Testing](testing.md)).
