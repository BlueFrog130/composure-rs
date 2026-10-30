# {{project-name}}

A Discord bot on Cloudflare Workers, built with [Composure](https://bluefrog130.github.io/composure-rs/).

## Develop

Install the tool and [cloudflared](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/),
then put your development application's secrets in `.env` and its id and your test server's in
`[package.metadata.composure.dev]` in `Cargo.toml`:

```sh
cargo install cargo-composure
cp .env.example .env             # DISCORD_PUBLIC_KEY, DISCORD_TOKEN
cargo composure dev
```

It serves the bot, opens a tunnel, sets the application's **Interactions Endpoint URL**, and on
every save rebuilds the bot and syncs its Commands to your test server.

`cargo test` runs the Commands without Discord.

## Deploy

```sh
npx wrangler deploy
npx wrangler secret put DISCORD_PUBLIC_KEY
npx wrangler secret put DISCORD_TOKEN
cargo run --bin sync             # every Command, globally
```

Then set the Interactions Endpoint URL to `https://{{project-name}}.<your subdomain>.workers.dev`.
See [Local development](https://bluefrog130.github.io/composure-rs/local_development.html) and
[Deploying](https://bluefrog130.github.io/composure-rs/deploying.html).
