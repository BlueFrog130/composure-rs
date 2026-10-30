<img src="../../assets/logo.svg" alt="Composure logo" width="64" height="64" align="right">

# Composure example bot on Cloudflare Workers

[![CI](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml)
[![Guide](https://img.shields.io/badge/guide-mdBook-blue?logo=mdbook)](https://bluefrog130.github.io/composure-rs/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE)

Serves the [example bot](../bot)'s shared commands from a Worker with `CloudflareInteractionBot`, plus what only Workers has, and a cron trigger using the Bot Client outside Interactions. `src/lib.rs` declares its App with `#[app(handlers(Bot, Note))] pub struct CloudflareBot;`.

[`src/notes.rs`](src/notes.rs): Workers KV

| Command | Shows |
|---|---|
| `/note save`, `/note show`, `/note forget` | A per-user note in the `NOTES` KV namespace with a 30-day TTL; `Ctx<Worker>`; KV futures made `Send` with `#[worker::send]` helpers that take the `Worker` State (ADR 0012) |

[`src/lib.rs`](src/lib.rs) `start` installs the adapter's `tracing` subscriber (feature `tracing`): `tracing::info!` in `/report`'s Background Work and the install welcome logs with the Interaction's or event's span fields, viewable with `npx wrangler tail` or in Workers Logs.

[`src/lib.rs`](src/lib.rs) `scheduled`: a cron trigger (`triggers.crons` in `wrangler.toml`) posts a daily heartbeat to the `STATUS_CHANNEL_ID` var's channel with `bot_client(&env)`; it needs the `DISCORD_APPLICATION_ID` var.

## Prerequisites

- Node (for `npx wrangler`) and `cargo install worker-build@0.8.7`
- A Discord application with a bot; note its **Application ID**, **Public Key**, and **Bot Token**

Worker config is `wrangler.toml`. `/note` needs a KV namespace: `npx wrangler kv namespace create NOTES`, then put its id in `kv_namespaces` (`wrangler dev` simulates it locally without one).

## Run locally

```sh
cd examples/cloudflare
cp .env.example .env      # gitignored: DISCORD_PUBLIC_KEY, DISCORD_TOKEN of a development application
cargo composure dev       # serves the Worker, opens a tunnel, repoints the application, syncs on every save
```

Set your own application and test server with `--application` and `--guild`, or in `[package.metadata.composure.dev]`. See [Local development](https://bluefrog130.github.io/composure-rs/local_development.html); `npx wrangler dev` (http://127.0.0.1:8787) still works on its own.

## Sync

The `sync-cloudflare` bin publishes the shared commands plus `/note`. It loads `.env` from this directory (copy `.env.example`; variables already set in the environment win):

```sh
cp .env.example .env   # fill in DISCORD_PUBLIC_KEY, DISCORD_TOKEN, DISCORD_APPLICATION_ID, COMPOSURE_SCOPE_DEV (where /mod goes)
cargo run -p composure_example_cloudflare --bin sync-cloudflare -- --dry-run # show the plan
cargo run -p composure_example_cloudflare --bin sync-cloudflare              # apply after confirming
cargo run -p composure_example_cloudflare --bin sync-cloudflare -- --yes     # apply without asking (CI)
cargo run -p composure_example_cloudflare --bin sync-cloudflare -- --guild <guild id>  # everything to one guild, for testing
```

`--help` lists every flag; outside a terminal it only applies with `--yes`. See the [`composure_sync` README](../../crates/sync/README.md).

`composure_sync` is a non-wasm dependency only, so it never reaches the Workers bundle.

The bundle is built for fast cold starts: `wasm-opt` settings in `Cargo.toml` and name stripping in `.cargo/config.toml`, explained in the adapter's [Cold starts](../../crates/adapters/cloudflare/README.md#cold-starts).

## Deploy

```sh
npx wrangler deploy
npx wrangler secret put DISCORD_PUBLIC_KEY
npx wrangler secret put DISCORD_TOKEN   # /whoami, /announce, /dm-me, /report, Thumbs up
```

Set the Interactions Endpoint URL to `https://composure-commands-example.<subdomain>.workers.dev`. For Webhook Events, set the Webhook Events URL to the same address plus `/events` and enable `Application Authorized`.

## Test

`cargo test -p composure_example_cloudflare` checks what Sync publishes. The shared commands are tested in [`examples/bot`](../bot); KV needs the Workers runtime, so `/note` is exercised with `npx wrangler dev`.
