# Composure commands example (Cloudflare)

A bot whose commands are each defined once with `#[command]` / `#[group]` / `#[derive(Choice)]` and collected with `commands!`. The same `Bot` serves interactions on Workers and is synced with Discord by `src/bin/sync.rs`.

| Command | Shows |
|---|---|
| `/hello` | `Ctx` (invoking user/member), user-install contexts |
| `/echo` | Options with constraints, optional Option, borrowed `&str` |
| `/ice-cream` | `#[derive(Choice)]` and its `name()`, `name` override, `Embed` response |
| `/greet` | `Ctx<impl Vars>`: a bounded State, `worker::Env` on Workers and a stand-in in tests |
| `/mod ban`, `/mod kick` | `#[group]` with `scope = "dev"` and permissions, `Ephemeral` replies, `HandlerError::user` |
| Avatar (user), Word count (message) | Context-menu commands |

Components go to `Components` via `Bot.with_fallback(Components)`.

## Prerequisites

- Node (for `npx wrangler`) and `cargo install worker-build@0.8.7`
- A Discord application with a bot; note its **Application ID**, **Public Key**, and **Bot Token**

Worker config is `wrangler.jsonc`.

## Run locally

```sh
cd adapters/cloudflare/commands-example
echo "DISCORD_PUBLIC_KEY=<public key>" > .dev.vars   # gitignored
npx wrangler dev                                     # http://127.0.0.1:8787
```

Discord needs a public HTTPS URL, so expose it with a tunnel (e.g. `cloudflared tunnel --url http://localhost:8787`) and set the application's **Interactions Endpoint URL** to the tunnel URL. Discord sends a signed Ping when you save; the bot must answer it for the save to succeed.

## Sync

The `sync` bin loads `.env` from this directory (copy `.env.example`; variables already set in the environment win):

```sh
cp .env.example .env   # fill in DISCORD_TOKEN, DISCORD_APPLICATION_ID, COMPOSURE_SCOPE_DEV (where /mod goes)
cargo run -p composure_example_commands --bin sync -- --dry-run # print the plan
cargo run -p composure_example_commands --bin sync              # apply after confirming
cargo run -p composure_example_commands --bin sync -- --guild <guild id>  # everything to one guild, for testing
```

`composure_api` is a non-wasm dependency only, so it never reaches the worker bundle.

## Deploy

```sh
npx wrangler deploy
npx wrangler secret put DISCORD_PUBLIC_KEY
```

Set the Interactions Endpoint URL to `https://composure-commands-example.<subdomain>.workers.dev`.

## Test

`cargo test -p composure_example_commands` sends signed interactions through `handle_interaction` on the host, with a stand-in for `worker::Env` as State.
