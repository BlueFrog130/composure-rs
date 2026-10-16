# Composure API

Basic interactions with Discord API, used at deploy time, never in the bot itself.

## Sync

`sync_from_env(&Bot)` makes Discord's commands match a Command Set: it prints a Sync Plan (creates, updates, deletes per target) and applies it after confirmation. Call it from a host-only bin:

```rust,ignore
// src/bin/sync.rs
fn main() -> std::process::ExitCode {
    composure_api::sync_from_env(&my_bot::Bot)
}
```

| Env var | |
|---|---|
| `DISCORD_TOKEN` | Bot token |
| `DISCORD_APPLICATION_ID` | Application id |
| `COMPOSURE_SCOPE_<NAME>` | Comma-separated guild ids for `#[command(scope = "name")]` (uppercased, `-` as `_`) |

Flags: `--dry-run` prints the plan only, `--yes` applies without asking (CI), `--guild <id>` sends every command to one guild and leaves global commands alone (testing).

Commands without a scope are global. Every guild listed in a `COMPOSURE_SCOPE_*` var is synced, so a guild whose commands were all removed gets cleared; guilds not listed are never touched. Targets without changes are skipped; changed ones are overwritten with one bulk request. `SyncConfig`, `plan`, and `apply` are available for other config sources.

## Live tests

`tests/regression.rs` holds a command suite covering every option type and field, subcommands and groups, localizations, permissions, NSFW, user/message commands, and contexts/integration types.

- **Offline** (always run): every suite command round-trips through the models unchanged.
- **Live** (`#[ignore]`d): one ordered test against the real Discord API. It clears all global and guild commands, publishes both suites, reads them back raw and through the models, exercises POST, `UpdateCommands`, an invalid command, and a bad token, then leaves the guild suite installed.

It **overwrites the app's global commands and the test guild's commands**, so use a dedicated test bot. Credentials come from `api/.env.test` (gitignored) or the environment:

```sh
DISCORD_TOKEN=...
DISCORD_APPLICATION_ID=...
DISCORD_GUILD_ID=...
```

```sh
cargo test -p composure_api --test regression -- --ignored --test-threads=1 --nocapture
```

It is local-only; CI runs just the offline tests.
