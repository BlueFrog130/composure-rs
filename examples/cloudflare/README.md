# Composure commands example (Cloudflare)

A bot whose commands are each defined once with `#[command]` / `#[group]` / `#[derive(Choice)]` and collected with `commands!`. The same `Bot` serves interactions on Workers and is synced with Discord by `src/bin/sync.rs`.

[`src/basics.rs`](src/basics.rs): answering directly

| Command | Shows |
|---|---|
| `/hello` | `Ctx` (invoking user/member), user-install contexts |
| `/echo` | Options with constraints, optional Option, borrowed `&str` |
| `/ice-cream` | `#[derive(Choice)]` and its `name()`, `name` override, `Embed` response |
| `/greet` | `Ctx<impl Vars>`: a bounded State, `Worker` on Workers and a stand-in in tests |
| `/mod ban`, `/mod kick` | `#[group]` with `scope = "dev"` and permissions, `Ephemeral` replies, `HandlerError::user` |
| Avatar (user), Word count (message) | Context-menu commands |

[`src/api.rs`](src/api.rs): Discord's REST API, through `Ctx<impl Platform>`

| Command | Shows |
|---|---|
| `/whoami` | `ctx.bot()` inline (bot token): Get Current User, `Response::model()` |
| `/remind` | `Ephemeral(ctx.defer(..))`, `DeferredCtx::sleep`, a follow-up message, result edits the original |
| `/countdown` | Editing the original response repeatedly from Background Work |
| `/announce` | Create Message as the bot (dev guild, `ManageMessages`); the response body is never parsed |
| Thumbs up (message) | Create Reaction on the target message |
| `/dm-me` | Create DM + Create Message; a REST failure turned into a `HandlerError::user` |
| `/report` | Bot Client in Background Work (Get Commands), a CSV file uploaded with the edit (multipart) |
| `/export` | A file in the initial response (the Adapter answers through the callback endpoint) |

[`src/interactive.rs`](src/interactive.rs): `#[component]` / `#[modal]` handlers, listed in `commands!` under `components` / `modals`

| Command | Shows |
|---|---|
| `/counter` | `#[component("counter")]` with the count as `custom_id` state (`counter_click::custom_id(n)`); `Update(..)` edits the clicked message |
| `/roll` | `ctx.defer_update(..)`: acknowledge without a loading message, edit the message from Background Work |
| `/colors` | A string select; `ComponentCtx::values()` |
| `/feedback` | `#[derive(Modal)]` form opened with `on_feedback::show()`, submitted to `#[modal("feedback")]` typed |

## Prerequisites

- Node (for `npx wrangler`) and `cargo install worker-build@0.8.7`
- A Discord application with a bot; note its **Application ID**, **Public Key**, and **Bot Token**

Worker config is `wrangler.jsonc`.

## Run locally

```sh
cd examples/cloudflare
echo "DISCORD_PUBLIC_KEY=<public key>" > .dev.vars   # gitignored
echo "DISCORD_TOKEN=<bot token>" >> .dev.vars        # for the REST commands
npx wrangler dev                                     # http://127.0.0.1:8787
```

Discord needs a public HTTPS URL, so expose it with a tunnel (e.g. `cloudflared tunnel --url http://localhost:8787`) and set the application's **Interactions Endpoint URL** to the tunnel URL. Discord sends a signed Ping when you save; the bot must answer it for the save to succeed.

## Sync

The `sync` bin loads `.env` from this directory (copy `.env.example`; variables already set in the environment win):

```sh
cp .env.example .env   # fill in DISCORD_TOKEN, DISCORD_APPLICATION_ID, COMPOSURE_SCOPE_DEV (where /mod goes)
cargo run -p composure_example_commands --bin sync -- --dry-run # show the plan
cargo run -p composure_example_commands --bin sync              # apply after confirming
cargo run -p composure_example_commands --bin sync -- --yes     # apply without asking (CI)
cargo run -p composure_example_commands --bin sync -- --guild <guild id>  # everything to one guild, for testing
```

`--help` lists every flag; outside a terminal it only applies with `--yes`. See the [`composure_sync` README](../../crates/sync/README.md).

`composure_sync` is a non-wasm dependency only, so it never reaches the worker bundle.

## Deploy

```sh
npx wrangler deploy
npx wrangler secret put DISCORD_PUBLIC_KEY
npx wrangler secret put DISCORD_TOKEN   # /whoami, /announce, /dm-me, /report, Thumbs up
```

Set the Interactions Endpoint URL to `https://composure-commands-example.<subdomain>.workers.dev`.

## Test

`cargo test -p composure_example_commands` sends signed interactions through `handle_interaction` on the host. The stand-in State implements `Platform` with a recording Transport, so REST calls are asserted without Discord, and runs Background Work on demand, as `wait_until` would after the response.
