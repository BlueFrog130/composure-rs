<img src="https://raw.githubusercontent.com/BlueFrog130/composure-rs/main/assets/logo.svg" alt="Composure logo" width="64" height="64" align="right">

# composure_sync

**`terraform plan` for your Discord slash commands.** Make Discord's stored commands match your
[Composure](https://github.com/BlueFrog130/composure-rs) bot's code: see exactly what will be
created, updated and deleted, confirm, done. Or let CI fail the build when they drift.

[![Crates.io](https://img.shields.io/crates/v/composure_sync.svg?logo=rust)](https://crates.io/crates/composure_sync)
[![Docs.rs](https://img.shields.io/docsrs/composure_sync?logo=docs.rs)](https://docs.rs/composure_sync)
[![CI](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/composure_sync.svg)](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.88+-orange.svg?logo=rust)](https://blog.rust-lang.org/)

- **One source of truth**: definitions come from your `#[command]` functions, never hand-written JSON
- **Safe by default**: a readable plan first; the prompt defaults to *No* when something would be deleted
- **CI-native**: `--check` exits 2 on drift, `--yes` deploys, and it never hangs waiting for input outside a terminal
- **Scopes**: keep admin and test commands in your own servers with `scope = "dev"`
- **Minimal requests**: unchanged targets are skipped, changed ones are one bulk overwrite each

## Usage

Requests go through Composure's `BotClient` over a blocking `reqwest` Transport, driven by `pollster`.

`sync_from_env(&Bot)` makes Discord's commands match an App's: it prints a Sync Plan (creates, updates, deletes per target) and applies it after confirmation. Call it from a host-only bin:

```rust,ignore
// src/bin/sync.rs
fn main() -> std::process::ExitCode {
    composure_sync::sync_from_env(&my_bot::Bot)
}
```

| Env var | |
|---|---|
| `DISCORD_TOKEN` | Bot token |
| `DISCORD_APPLICATION_ID` | Application id |
| `COMPOSURE_SCOPE_<NAME>` | Comma-separated guild ids for `#[command(scope = "name")]` (uppercased, `-` as `_`) |

In a terminal it shows the plan (green `+` create, yellow `~` update, red `-` delete) and asks before applying; the prompt defaults to No when something would be deleted. Outside a terminal, or when `CI` is set, it never prompts: it applies only with `--yes` and otherwise fails without changing anything.

| Flag | |
|---|---|
| `--dry-run` | Show the plan only |
| `--check` | Show the plan; exit 2 if Discord is out of date (CI drift check) |
| `-y`, `--yes` | Apply without asking (CI deploys) |
| `--guild <id>` | Send every command to one guild, leaving global commands alone (testing) |
| `--color <auto\|always\|never>` | `always` keeps colors in CI logs; `NO_COLOR` / `CLICOLOR_FORCE` are honored too |

Exit status: 0 synced or up to date, 1 failed, cancelled, or needs `--yes`, 2 out of date (`--check`).

```yaml
# GitHub Actions
- run: cargo run --bin sync -- --yes --color always
  env:
    DISCORD_TOKEN: ${{ secrets.DISCORD_TOKEN }}
    DISCORD_APPLICATION_ID: ${{ vars.DISCORD_APPLICATION_ID }}
```

The CLI (clap + cliclack) is the default `cli` feature; with `default-features = false` only `SyncConfig`, `plan`, and `apply` remain.

Commands without a scope are global. Every guild listed in a `COMPOSURE_SCOPE_*` var is synced, so a guild whose commands were all removed gets cleared; guilds not listed are never touched. Targets without changes are skipped; changed ones are overwritten with one bulk request. `SyncConfig`, `plan`, and `apply` are available for other config sources or a custom front end.

## Development: live API tests

`tests/api_regression.rs` holds a command suite covering every option type and field, subcommands and groups, localizations, permissions, NSFW, user/message commands, and contexts/integration types.

- **Offline** (always run): every suite command round-trips through the types unchanged.
- **Live** (`#[ignore]`d): one ordered test against the real Discord API. It clears all global and guild commands, publishes both suites, reads them back raw and through the types, exercises POST, the builder, an invalid command, and a bad token, then leaves the guild suite installed.

It **overwrites the app's global commands and the test guild's commands**, so use a dedicated test bot. Credentials come from `crates/sync/.env.test` (gitignored) or the environment:

```sh
DISCORD_TOKEN=...
DISCORD_APPLICATION_ID=...
DISCORD_GUILD_ID=...
```

```sh
cargo test -p composure_sync --test api_regression -- --ignored --test-threads=1 --nocapture
```

## License

Licensed under the [Apache License, Version 2.0](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE).
