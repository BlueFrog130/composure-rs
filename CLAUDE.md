# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Composure: Rust framework for HTTP-interaction-only Discord bots meant to run on the edge (serverless/WASM), no gateway. Cargo workspace; root crate is the core library.

## Commands

```sh
cargo build --workspace
cargo test --workspace
cargo test -p composure_commands            # one crate
cargo test -p composure validate_request_ok # one test
cargo clippy --workspace
cargo fmt --all
```

**`composure_api` tests hit the live Discord API** (create/overwrite real commands). They read `api/.env.test` (gitignored) with `DISCORD_TOKEN`, `DISCORD_APPLICATION_ID`, `DISCORD_GUILD_ID`, and panic without it. Exclude them for offline runs: `cargo test --workspace --exclude composure_api`.

## Workspace crates

| Crate | Path | Role |
|---|---|---|
| `composure` | `/` | Core: Discord models (serde), ed25519 request validation, `command` runtime (`InteractionHandler`, `Ctx`, Command Set traits), `definition` models. Must stay WASM-friendly and small — no async runtime, no HTTP client. `macros` feature re-exports `composure_macros`. |
| `composure_macros` | `macros/` | Proc macros: `#[command]`, `#[group]`, `#[derive(Choice)]`. `commands!` is a `macro_rules!` in core. Runtime + trybuild UI tests in `macros/tests`. |
| `composure_commands` | `commands/` | Definition builder (`CommandsBuilder` → `CommandBuilder` → `SubcommandBuilder`/group); re-exports `composure::definition`. |
| `composure_api` | `api/` | Blocking `reqwest` Discord REST client + Sync (`sync_from_env`, `plan`, `apply`). Deploy-time only. |
| `composure_adapter_cloudflare` | `adapters/cloudflare/` | Cloudflare Workers adapter (`worker` crate). `example/` = low-level regression bot, `commands-example/` = `#[command]` bot with `sync` bin. |

Release profile is size-optimized (`opt-level = "z"`, LTO) because worker bundle size matters.

## Architecture

**Request flow (adapter):** read body bytes → validate `X-Signature-Ed25519` / `X-Signature-Timestamp` against `DISCORD_PUBLIC_KEY` secret via `composure::auth::validate_request` → deserialize `Interaction` → `composure::command::dispatch` to an `InteractionHandler<S>` (S = adapter State, `worker::Env` on Cloudflare) → return `InteractionResponse` as JSON. Ping → Pong in core. Handler `Err(HandlerError)` is logged and answered with `HandlerError::response()` (ephemeral).

**Commands:** `#[command] fn x(ctx: Ctx, opts...)` keeps the fn and adds a same-named empty braced struct (type namespace only) implementing `CommandDefinition` (+ `SubcommandDefinition` for chat input) and `Command<S>`. Option params go through `OptionType` (extract + definition); compile-time rules are `const _: () = assert!(...)` blocks. `#[group]` routes via `Invocation::route`. `commands! { pub struct Bot = [...]; }` implements `CommandSet<S>`, `InteractionHandler<S>`, `CommandSetDefinition`, and an inherent `with_fallback`. Async fns in traits are native (no `Send`, static dispatch) — keep it that way for Workers.

Adapters are the integration layer; new platforms go under `adapters/<name>` and reuse core validation + models.

**Models (`src/models/`)** are split by direction:
- `deserialize/` — inbound Discord payloads (interaction, channel, member, message, role, ...)
- `serialize/` — outbound (`InteractionResponse`)
- `common/` — both directions (embed, component, permissions, snowflake, ...)

All submodules are glob re-exported, so everything is addressed as `composure::models::X`.

**Serde conventions:**
- `Interaction` uses a manual `Deserialize` that peeks the `type` field and dispatches to `DataInteraction<T>` variants (`ApplicationCommandInteraction`, `MessageComponentInteraction`, `ModalSubmitInteraction` are type aliases). Follow this pattern for type-discriminated unions.
- `TypeField<const T: u8>` is a zero-sized marker that serializes as the constant and fails deserialization on mismatch — used as a `t` field to tag struct variants inside `#[serde(untagged)]`-style enums (see `commands/src/command/model.rs`).
- Integer enums use `serde_repr`; flags use `bitflags`.
- `Snowflake` is a custom struct (decomposed timestamp/worker/process/increment) serialized as a string.

## Notes

- Glossary in `CONTEXT.md` (Command, Command Set, Option, Scope, Sync, ...); decisions in `docs/adr/`.
- All crates share version `0.1.0`; bump together (path deps also pin `version`).
- Regenerate trybuild expectations after changing macro errors: `TRYBUILD=overwrite cargo test -p composure_macros --test commands`, then review the `.stderr` diffs.

## Agent skills

### Issue tracker

GitHub Issues on BlueFrog130/composure-rs, via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `CONTEXT.md` plus `docs/adr/`. See `docs/agents/domain.md`.
