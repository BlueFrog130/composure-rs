# Deploying

A deploy has two halves: the bot's code, served where Discord can reach it, and its Commands,
registered with Discord by [Sync](getting_started.md#syncing-commands). Deploy the code first,
then sync: a new Command synced before the code that answers it is "Unknown command." for a
moment, while the other order only shows the old Commands a little longer.

## Cloudflare Workers

```sh
npx wrangler deploy
npx wrangler secret put DISCORD_PUBLIC_KEY   # once; DISCORD_TOKEN too for the bot client
cargo run --bin sync                         # shows the plan, applies after confirming
```

The Worker answers at `https://<name>.<your subdomain>.workers.dev`, or a custom domain from the
dashboard; put that URL in the Developer Portal as the **Interactions Endpoint URL**. Secrets
(`wrangler secret put`) hold the public key and bot token; plain settings go in `[vars]` in
`wrangler.toml`, next to bindings like KV. See
[`composure_adapter_cloudflare`](https://docs.rs/composure_adapter_cloudflare) for the build
settings that keep cold starts short.

### From CI

A GitHub Actions workflow that tests, deploys and syncs on every push to `main`, as the
repository's own bot does:

```yaml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - run: cargo install worker-build@0.8.7 --locked
      - run: cargo test
      - uses: cloudflare/wrangler-action@v4
        with:
          apiToken: ${{ secrets.CLOUDFLARE_API_TOKEN }}
          accountId: ${{ secrets.CLOUDFLARE_ACCOUNT_ID }}
          command: deploy
      - run: cargo run --bin sync -- --yes
        env:
          DISCORD_TOKEN: ${{ secrets.DISCORD_TOKEN }}
          DISCORD_APPLICATION_ID: ${{ secrets.DISCORD_APPLICATION_ID }}
```

`CLOUDFLARE_API_TOKEN` needs the *Workers Scripts: Edit* permission. Outside a terminal, Sync
only applies with `--yes`; `cargo run --bin sync -- --check` in pull requests fails (exit status
2) when Discord's Commands differ from the App.

## A server

The tokio template's `Dockerfile` builds a small image:

```sh
docker build -t my-bot .
docker run -p 8080:8080 -e DISCORD_PUBLIC_KEY=... -e DISCORD_TOKEN=... my-bot
```

Run it anywhere that runs containers. What the Adapter expects from its host:

- **TLS in front.** It speaks plain HTTP on `$PORT` (8080). Put a TLS-terminating proxy or load
  balancer in front (most container platforms provide one), and use its HTTPS URL as the
  Interactions Endpoint URL. With Caddy, for one:

  ```text
  bot.example.com {
      reverse_proxy localhost:8080
  }
  ```

- **Health checks.** `GET /health` answers 200.
- **Graceful stops.** On SIGTERM (or Ctrl-C) it stops accepting requests and waits up to 30
  seconds for in-flight requests and Background Work, such as deferred answers, before
  exiting; `.with_grace(..)` changes the wait. Give the platform's stop timeout at least as
  long.
- **Configuration from the environment.** `DISCORD_PUBLIC_KEY`, `DISCORD_TOKEN`,
  `DISCORD_APPLICATION_ID` and `PORT`, or [Credentials](platforms.md#credentials) loaded any
  other way.

Each Interaction runs on its own task, so one replica handles many at once; several replicas
behind the load balancer need nothing shared, since Interactions carry their own state.

## Before going live

- Use the production application's public key and token, not the development one's.
- Sync without `--guild`, so the Commands are global and follow their [Scopes](commands.md#scopes).
- Check the Endpoint URL saves: Discord tests it, so a mistake shows at once.
