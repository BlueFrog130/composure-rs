# {{project-name}}

A Discord bot on a server with tokio, built with [Composure](https://bluefrog130.github.io/composure-rs/).

## Develop

```sh
cp .env.example .env             # DISCORD_PUBLIC_KEY, DISCORD_TOKEN, DISCORD_APPLICATION_ID
cargo run                        # http://localhost:8080
cloudflared tunnel --url http://localhost:8080
```

Put the tunnel's HTTPS URL in the Developer Portal as the **Interactions Endpoint URL**, then
register the Commands in your test server, where they show up at once:

```sh
cargo run --bin sync -- --guild <your test server's id>
```

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
