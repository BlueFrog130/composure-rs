# Composure API

Basic interactions with Discord API. Should be used to update commands for bot at publish time.

## Live tests

The tests in this crate call the real Discord API (they create and overwrite commands), so they are `#[ignore]`d by default. To run them, create `api/.env.test`:

```sh
DISCORD_TOKEN=...
DISCORD_APPLICATION_ID=...
DISCORD_GUILD_ID=...
```

then run:

```sh
cargo test -p composure_api -- --ignored
```
