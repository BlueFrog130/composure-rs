<img src="../../assets/logo.svg" alt="Composure logo" width="64" height="64" align="right">

# Composure bot

Composure's own Discord bot, built with Composure and served from Cloudflare Workers. It
announces each release, pages through the changelog and searches the guide. It's a real
deployment rather than an example; the [examples](../../examples) teach the framework one
feature at a time.

| Command | What it does | Shows |
|---|---|---|
| `/changelog [version]` | One release per page: newer/older buttons, a jump menu, a link to GitHub | A `#[view]` with async inline handlers, state in `custom_id`s, re-render vs. ephemeral copy |
| `/compare <from> [to]` | The same pages, limited to a range | Two autocompleted Options, one View for both Commands |
| *Explain release* (message) | Opens the changelog at the version a message mentions | Message context menus |
| `/notify setup\|remove\|status\|preview` | Release announcements in a server's channel | A Group, `channel_types`, `permissions`, guild-only contexts, REST errors turned into user messages |
| `/docs <topic>` | Guide sections and API items, suggested while typing | Sync autocomplete over a `build.rs` index of `book/` |
| `/crate [name]` | Version and downloads from crates.io | `ctx.defer`, a Choice enum, KV cache with a TTL |
| `/feedback` | A form posted to the maintainers, plus a prefilled GitHub issue link | A modal View with typed submit parameters |
| `/about`, `/invite` | What this is; install links for servers and for users | Components V2, link buttons |

The bot also logs every install to a private channel and DMs whoever adds it to a server
(the `APPLICATION_AUTHORIZED` Webhook Event).

## Next to composurejs's bot

[composurejs's bot](https://github.com/BlueFrog130/composure-js/tree/main/bots/composure) offers
the same Commands for the TypeScript port (`/package` for `/crate`) in the same home server. Each is
its own Discord application, so Commands, components and webhooks never cross; name them
**composure-rs** and **composurejs** in the Developer Portal, since Discord labels every Command and
context menu entry with the app's name. Otherwise they keep apart by:

- **Explain release**: versions named for another package (`composurejs v0.2.0`,
  `composure_macros/v0.1.4`) are skipped, so a composurejs announcement never opens this changelog.
- **Announcement channels**: one per bot (`#rust-releases`, `#js-releases`). `/notify` only sees
  follower webhooks whose source is its own channel, so neither bot removes the other's.
- **Cloudflare**: the Workers (`composure-bot`, `composurejs-bot`) and KV namespaces are separate;
  their keys are the same, so they must never share a namespace.
- **Color**: this bot's rust orange (`0xd34516`) against composurejs's TypeScript blue (`0x3178c6`).
- **Footer**: every panel ends with `-# composure-rs · Rust` (composurejs: `-# composurejs · TypeScript`),
  for where the color doesn't show.

The feedback and install-log channels may be shared: each post's author is the bot that sent it.
The composurejs bot's README has the full table.

## How announcements work

- Every 15 minutes, the `scheduled` handler in [`src/worker.rs`](src/worker.rs) fetches the
  repository's GitHub Releases and stores them in KV, where every command reads them. knope puts
  each version's changelog section in its release.
- Each release newer than the last one announced is posted to the home server's announcement
  channel (`ANNOUNCE_CHANNEL_ID`) and published. Releases are announced oldest first. The first
  run announces every release, so the announcement channel starts with the full history.
- `/notify setup #channel` makes the server follow that announcement channel, so Discord
  delivers the published posts. Nothing is stored per server: `/notify status` and `remove` read
  the server's channel follower webhooks. Adding the bot to a server asks only for **Manage
  Webhooks**.

## Code

[`src/lib.rs`](src/lib.rs) declares the App. Handlers take `Ctx<impl Services>`
([`src/services.rs`](src/services.rs)): a `Platform` with the bot's `Config` as Bot Data and a
key-value `Store`. On Workers that's `Worker<Config>` with Workers KV
([`src/worker.rs`](src/worker.rs)). [`tests/offline.rs`](tests/offline.rs) uses a stand-in with
a map for KV and one Transport that answers Discord, GitHub and crates.io. GitHub and crates.io
are called over the Platform's Transport, so tests intercept those calls like Discord's.

Every message is built in `rsx!` from the tone containers in [`src/ui.rs`](src/ui.rs):
`<ui::Primary>`, `<ui::Secondary>`, `<ui::Success>`, `<ui::Warning>` and `<ui::Error>`, each with
an optional `title`. They set their accent color from the consts there, so replies and
messages sent through REST (`ui::send`: announcements, feedback, the install log) look the same.
Handler errors are answered in the same style by `ui::render_error`, the App's `errors = ..`.

## Set up

There are two environments, each with its own Discord application and home-server channels
under `[env.<env>]` in `wrangler.toml`:

- **`production`** is deployed as the Worker `composure-bot`.
- **`dev`** only runs locally with `wrangler dev`, which simulates its KV namespace (no id).

For production you need:

- Node (for `npx wrangler`) and `cargo install worker-build@0.8.7`
- A Discord application with a bot
- A server with an announcement channel, plus two private channels (feedback, install log)

Then:

1. Create the KV namespace with `npx wrangler kv namespace create COMPOSURE`, and put the id
   under `[[env.production.kv_namespaces]]`.
2. Fill in `[env.production.vars]`: the application id, and the announcement, feedback and log
   channel ids.
3. Add the bot to the home server with **Send Messages** in the announcement, feedback and log
   channels. Publishing its own posts needs nothing more.
4. Set the secrets:

   ```sh
   npx wrangler secret put DISCORD_PUBLIC_KEY --env production
   npx wrangler secret put DISCORD_TOKEN --env production
   npx wrangler secret put GITHUB_TOKEN --env production   # fine-grained, read-only Contents
   ```

5. Deploy and sync:

   ```sh
   npx wrangler deploy --env production
   cp .env.example .env   # the production application's DISCORD_TOKEN, DISCORD_APPLICATION_ID
   cargo run -p composure_bot --bin sync-composure-bot
   ```

6. In the Developer Portal:
   - Set the **Interactions Endpoint URL** to the Worker's URL
     (`https://composure-bot.<subdomain>.workers.dev`).
   - Set the **Webhook Events URL** to the same URL plus `/events`, and enable **Application
     Authorized**.
   - Under Installation, enable both **Guild Install** and **User Install**.

The [deploy workflow](../../.github/workflows/bot.yml) repeats step 5 on every push to main
that touches the bot or the crates, and after each release. It needs the repository secrets
`CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`, `DISCORD_TOKEN` and `DISCORD_APPLICATION_ID`
(the production application's).

## Run locally

The `dev` environment, against the dev application:

```sh
cd bots/composure
printf 'DISCORD_PUBLIC_KEY=...
DISCORD_TOKEN=...
GITHUB_TOKEN=...
' > .dev.vars   # gitignored
npx wrangler dev --env dev --test-scheduled   # http://127.0.0.1:8787; GET /__scheduled runs the cron
```

Discord needs a public HTTPS URL, so expose it with a tunnel (e.g.
`cloudflared tunnel --url http://localhost:8787`) and set it as the dev application's
Interactions Endpoint URL. Sync the dev application's commands with its token in `.env`.

## Test

```sh
cargo test -p composure_bot
```

The tests cover every command, the page turns, `/notify`'s webhook calls, the cron's
announcements, the install events and Sync's definitions. KV itself needs the Workers runtime,
so it's exercised with `wrangler dev`.
