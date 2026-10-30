# {{project-name}}

A Discord bot on a server with tokio, built with [Composure](https://bluefrog130.github.io/composure-rs/).

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
docker build -t {{project-name}} .
docker run -p 8080:8080 --env-file .env {{project-name}}
cargo run --bin sync             # every Command, globally
```

It speaks plain HTTP: serve it behind a TLS-terminating proxy and use that HTTPS URL as the
Interactions Endpoint URL. See
[Local development](https://bluefrog130.github.io/composure-rs/local_development.html) and
[Deploying](https://bluefrog130.github.io/composure-rs/deploying.html).
