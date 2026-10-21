# {{project-name}}

A Discord bot on Cloudflare Workers, built with [Composure](https://bluefrog130.github.io/composure-rs/).

## Develop

```sh
cp .dev.vars.example .dev.vars   # DISCORD_PUBLIC_KEY, DISCORD_TOKEN
npx wrangler dev                 # http://localhost:8787
cloudflared tunnel --url http://localhost:8787
```

Put the tunnel's HTTPS URL in the Developer Portal as the **Interactions Endpoint URL**, then
register the Commands in your test server, where they show up at once:

```sh
cp .env.example .env             # DISCORD_TOKEN, DISCORD_APPLICATION_ID
cargo run --bin sync -- --guild <your test server's id>
```

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
