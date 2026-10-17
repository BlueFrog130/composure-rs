# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Composure: Rust framework for HTTP-interaction-only Discord bots meant to run on the edge (serverless/WASM), no gateway. Virtual Cargo workspace; crates under `crates/`, examples under `examples/`.

## Commands

```sh
cargo build --workspace
cargo test --workspace
cargo test -p composure_commands            # one crate
cargo test -p composure validate_request_ok # one test
cargo clippy --workspace
cargo fmt --all
```

`cargo test --workspace` is offline. Live Discord API tests are `#[ignore]`d (`composure_sync --test api_regression`; they overwrite real commands and read `crates/sync/.env.test`, gitignored, with `DISCORD_TOKEN`, `DISCORD_APPLICATION_ID`, `DISCORD_GUILD_ID`).

REST Endpoints are checked against Discord's OpenAPI spec vendored in `crates/composure/spec/` (pinned commit in `PINNED`; bump with `scripts/update-spec.sh`): `cargo test -p composure --features full --lib rest::spec -- --nocapture` also prints unimplemented routes.

## Workspace crates

| Crate | Path | Role |
|---|---|---|
| `composure` | `crates/composure/` | Core: Discord types (serde), ed25519 request validation, `command` runtime (`InteractionHandler`, `Ctx`, Command Set traits), `definition` models, `rest` client. Must stay WASM-friendly and small — no async runtime, no HTTP implementation. `macros` feature re-exports `composure_macros`. |
| `composure_macros` | `crates/macros/` | Proc macros: `#[command]`, `#[group]`, `#[derive(Choice)]`. `commands!` is a `macro_rules!` in core. Runtime + trybuild UI tests in `crates/macros/tests`. |
| `composure_commands` | `crates/commands/` | Definition builder (`CommandsBuilder` → `CommandBuilder` → `SubcommandBuilder`/group); re-exports `composure::definition`. |
| `composure_sync` | `crates/sync/` | Sync (`plan`, `apply`; `sync_from_env` CLI via clap + cliclack behind default `cli` feature) on core's `BotClient` + `reqwest` Transport via `pollster`. Deploy-time only. |
| `composure_adapter_cloudflare` | `crates/adapters/cloudflare/` | Cloudflare Workers adapter (`worker` crate). `bench/` (own workspace member, `exclude`d from the adapter package) = `validate_request` dalek vs `web-crypto` benchmark on `wrangler dev` (`cargo test -p composure_bench_cloudflare --test wrangler --release -- --ignored --nocapture`). |
| `composure_example_commands` | `examples/cloudflare/` | Example bot on Workers with a `sync` bin: `basics` (answers), `api` (REST, defer, files), `interactive` (buttons, modals). `tests/offline.rs` uses a `Platform` stand-in with a recording Transport; `tests/regression.rs` runs signed fixtures for every interaction shape. |

Release profile is size-optimized (`opt-level = "z"`, LTO) because worker bundle size matters.

## Architecture

**Request flow (adapter):** read body bytes → validate `X-Signature-Ed25519` / `X-Signature-Timestamp` against `DISCORD_PUBLIC_KEY` secret via async `composure::auth::validate_request` (`ed25519-dalek`; opt-in `web-crypto` feature uses `crypto.subtle` on `wasm32-unknown-unknown`) → deserialize `Interaction` → `composure::command::dispatch` to an `InteractionHandler<S>` (S = adapter State, `Worker` = `Env` + `Context` on Cloudflare) → return `InteractionResponse` as JSON (responses with files go through the callback endpoint, then 202). Ping → Pong in core. Handler `Err(HandlerError)` is logged and answered with `HandlerError::response()` (ephemeral).

**Defer:** a State implementing `command::Platform` (Transport, bot token, `spawn`) enables `ctx.client()`, `ctx.bot()`, `ctx.defer(async |ctx: DeferredCtx| ..)`. `defer` spawns the Background Work (closure, then edit original with its output) through `Platform::spawn` (`wait_until`) and returns type 5; dispatch signatures are unchanged (ADR 0006). Commands that use REST take `Ctx<impl Platform>` so tests can pass a stand-in State.

**Commands:** `#[command] fn x(ctx: Ctx, opts...)` keeps the fn and adds a same-named empty braced struct (type namespace only) implementing `CommandDefinition` (+ `SubcommandDefinition` for chat input) and `Command<S>`. Option params go through `OptionType` (extract + definition); compile-time rules are `const _: () = assert!(...)` blocks. `#[group]` routes via `Invocation::route`. `commands! { pub struct Bot = [...]; }` implements `CommandSet<S>`, `InteractionHandler<S>`, `CommandSetDefinition`, and an inherent `with_fallback`. Async fns in traits are native (no `Send`, static dispatch) — keep it that way for Workers.

**Components/modals:** `#[component("prefix")] fn f(ctx: ComponentCtx, state..)` and `#[modal("prefix")] fn f(ctx: ModalCtx, form: F, state..)` (`F: #[derive(Modal)]`: each field is a Label around the input its attribute names — `input`/`select`/`user_select`/.../`radio`/`checkboxes`/`checkbox`/`file_upload` — and its type, via `command::ModalValue`, decides required-ness and how the value is read; `#[derive(Choice)]` enums supply options via `ModalChoice`) add a same-named struct implementing `ComponentHandler<S>` / `ModalHandler<S>` plus `f::custom_id(state..)` (and `f::show(state..)` for modals). State is encoded in the `custom_id` by `command::custom_id` (serde, `prefix:field:field`, `\` escapes, ADR 0007). `commands! { ..; components = [..]; modals = [..]; }` routes by prefix (`CommandSet::dispatch_component/dispatch_modal`, also used by `WithFallback`). `Ctx<'a, S, D>` is generic over interaction data; `ComponentCtx`/`ModalCtx` are aliases. `#[autocomplete] fn f(ctx: Ctx, input: &str) -> Vec<..>` implements `Autocomplete<S>`; `#[option(.., autocomplete = f)]` sets `autocomplete: true` and generates `Command::autocomplete` routing the focused Option (Groups forward via `Invocation::route`); `commands!` exposes it as `CommandSet::dispatch_autocomplete`. Themes: `commands! { ..; theme = THEME; }` sets `CommandSet::THEME` / `InteractionHandler::theme()`; `ctx.theme()` (default palette when unset) gives `command::Theme` helpers (`embed`, `success`, ...); `dispatch` and deferred edits fill unset embed color/author/footer and top-level `Container` `accent_color` (`Unthemed` opts out), adapters render errors with `HandlerError::response_with(handler.theme())`; REST bodies are untouched (ADR 0009). Webhook Events: `types::{WebhookEventPayload, Event}` (unknown/mismatched → `Event::Unknown`, never an error), `command::EventHandler<S>` (per-event methods default to no-op, `handle` routes), `commands! { ..; events = X; }` (else ignored); the adapter serves `/events`, ACKs 204 and runs the handler via `spawn_event` (ADR 0008). Components: `types::Component` covers every type in the reference (layout: `ActionRow`, `Section`, `Container`, `Separator`, `Label`; content: `TextDisplay`, `Thumbnail`, `MediaGallery`, `FileComponent`, `UnfurledMedia`; inputs), each with an optional `id` (`Component::id`/`with_id`); unknown types deserialize to `Component::Unknown(Value)`, never an error. `IntoResponse for Container` / `InteractionResponse::respond_with_components` set `MessageFlags::IsComponentsV2` (no content/embeds alongside); message `components` fields are `Vec<Component>`. Regenerate trybuild expectations for new macro errors.

Adapters are the integration layer; new platforms go under `crates/adapters/<name>` and reuse core validation + types.

**Types (`crates/composure/src/types/`)** are one file per resource (interaction, message, channel, member, role, embed, component, ...), shared by Interactions, Responses and REST. All are glob re-exported, so everything is addressed as `composure::types::X`.

**REST (`crates/composure/src/rest/`, `rest` / `rest-*` / `tier1` / `full` features; one module + feature per resource, registered in `rest.rs`, `rest/spec.rs` `modules()`, and `Cargo.toml` tiers):** async clients over an Adapter-provided `Transport` (`send` + `sleep`; own `Request`/`RawResponse` types, core encodes JSON/multipart). `InteractionClient` (interaction token, no auth) and `BotClient` (bot token; resource Endpoints added as `impl BotClient<T>` blocks in per-feature modules). Each module declares its routes with `routes! { NAME = Method "/spec/{path}"; }` — paths copied from the spec so the spec check matches. Endpoints take borrowed body structs and return lazily-decoded `Response<T>` (`()` for 204). 429s are waited out via `Transport::sleep` up to `with_max_rate_limit_wait` (2s default, Sync uses 60s). Tests use `rest::testing::MockTransport`.

**Serde conventions:**
- `Interaction` uses a manual `Deserialize` that reads the body once into a private `Envelope` (common fields typed, `data` kept as `Box<RawValue>`), then parses `data` by `type` into `DataInteraction<T>` variants (`ApplicationCommandInteraction`, `MessageComponentInteraction`, `ModalSubmitInteraction` are type aliases). Options do the same via `RawOption`. This is the hot path: don't go through `serde_json::Value` or `#[serde(flatten)]` for type-discriminated unions.
- `OptionType<'a>` / `CommandTarget<'a>` carry the Interaction's lifetime so `&str`, `&User`, `&Message`, ... parameters borrow instead of clone.
- `TypeField<const T: u8>` is a zero-sized marker that serializes as the constant and fails deserialization on mismatch — used as a `t` field to tag struct variants inside `#[serde(untagged)]`-style enums (see `crates/commands/src/command/model.rs`).
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
