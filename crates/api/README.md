# Composure API

Blocking Discord REST client (`DiscordClient`, `UpdateCommands`), used at deploy time, never in the bot itself. Sync lives in [`composure_sync`](../sync).

## Live tests

`tests/regression.rs` holds a command suite covering every option type and field, subcommands and groups, localizations, permissions, NSFW, user/message commands, and contexts/integration types.

- **Offline** (always run): every suite command round-trips through the models unchanged.
- **Live** (`#[ignore]`d): one ordered test against the real Discord API. It clears all global and guild commands, publishes both suites, reads them back raw and through the models, exercises POST, `UpdateCommands`, an invalid command, and a bad token, then leaves the guild suite installed.

It **overwrites the app's global commands and the test guild's commands**, so use a dedicated test bot. Credentials come from `crates/api/.env.test` (gitignored) or the environment:

```sh
DISCORD_TOKEN=...
DISCORD_APPLICATION_ID=...
DISCORD_GUILD_ID=...
```

```sh
cargo test -p composure_api --test regression -- --ignored --test-threads=1 --nocapture
```

It is local-only; CI runs just the offline tests.
