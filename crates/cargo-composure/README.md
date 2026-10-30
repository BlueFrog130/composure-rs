# cargo-composure

`cargo composure dev` is the whole development loop for a
[Composure](https://bluefrog130.github.io/composure-rs/) Discord bot: it serves the bot, opens a
tunnel, points your development application at it, and keeps the bot and its Commands up to date on
every save.

```sh
cargo install cargo-composure
```

```toml
# Cargo.toml: neither id is a secret
[package.metadata.composure.dev]
application = "1100146565504565411" # your development application
guild = "798662131062931547"        # your test server
```

```sh
cargo composure dev
```

```txt
[composure] bot serving (6.1s)
[composure] dev Sync to guild 798662131062931547: 2 created
[composure] tunnel connected https://example-words-here.trycloudflare.com (6.2s)
[composure] https://example-words-here.trycloudflare.com answers through the tunnel (6.9s)
[composure] Dev Application 1100146565504565411 -> https://example-words-here.trycloudflare.com (8.1s)
```

It works on Cloudflare Workers (a package with a `wrangler.toml`) and hosted servers (the tokio
adapter), and needs [cloudflared](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/)
for its default tunnel. The application is only changed when `DISCORD_TOKEN` is that application's
token, so a production token never repoints production.

See [Local development](https://bluefrog130.github.io/composure-rs/local_development.html).
