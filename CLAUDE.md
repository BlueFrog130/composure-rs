# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

Composure: Rust framework for HTTP-interaction-only Discord bots meant to run on the edge (serverless/WASM) or a hosted server (tokio), no gateway. Virtual Cargo workspace; crates under `crates/`, examples under `examples/`.

## Commands

```sh
cargo build --workspace
cargo test --workspace
cargo test -p composure_commands            # one crate
cargo test -p composure validate_request_ok # one test
cargo clippy --workspace
cargo fmt --all
cargo test -p composure_example_bot --release --test perf -- --ignored --nocapture  # per-path allocations/us
```

`cargo test --workspace` is offline. Live Discord API tests are `#[ignore]`d (`composure_sync --test api_regression`; they overwrite real commands and read `crates/sync/.env.test`, gitignored, with `DISCORD_TOKEN`, `DISCORD_APPLICATION_ID`, `DISCORD_GUILD_ID`).

REST Endpoints are checked against Discord's OpenAPI spec vendored in `crates/composure/spec/` (pinned commit in `PINNED`; bump with `scripts/update-spec.sh`): `cargo test -p composure --features full --lib rest::spec -- --nocapture` also prints unimplemented routes.

## Workspace crates

| Crate | Path | Role |
|---|---|---|
| `composure` | `crates/composure/` | Core: Discord types (serde), ed25519 request validation, `adapter` (the request pipeline every Adapter shares), `command` runtime (`InteractionHandler`, `Ctx`, `Route` traits), `definition` models, `rest` client. Must stay WASM-friendly and small — no async runtime, no HTTP implementation. `macros` feature re-exports `composure_macros`. |
| `composure_macros` | `crates/macros/` | Proc macros: `#[app]`, `#[command]`, `#[group]`, `#[component]`, `#[modal]`, `#[derive(Choice)]`, `#[view]` + `rsx!`. Runtime + trybuild UI tests in `crates/macros/tests`. |
| `composure_commands` | `crates/commands/` | Definition builder (`CommandsBuilder` → `CommandBuilder` → `SubcommandBuilder`/group); re-exports `composure::definition`. |
| `composure_sync` | `crates/sync/` | Sync (`plan`, `apply`; `sync_from_env` CLI via clap + cliclack behind default `cli` feature) on core's `BotClient` + `reqwest` Transport via `pollster`. Deploy-time only. |
| `composure_adapter_cloudflare` | `crates/adapters/cloudflare/` | Cloudflare Workers adapter (`worker` crate). `bench/` (own workspace member, `exclude`d from the adapter package) = `validate_request` dalek vs `web-crypto` benchmark on `wrangler dev` (`cargo test -p composure_bench_cloudflare --test wrangler --release -- --ignored --nocapture`). |
| `composure_adapter_tokio` | `crates/adapters/tokio/` | Hosted-server adapter: hyper (http1) on tokio, each Interaction its own `tokio::spawn`ed task (panic → 500), `Host<T>` State (user data + token + `HttpTransport` over async reqwest/rustls + `TaskTracker` for Background Work), `GET /health`, 1 MiB body cap (413), graceful shutdown (Ctrl-C/SIGTERM or `with_shutdown`, 30s grace). `tests/server.rs` runs over a real socket. |
| `composure_example_bot` | `examples/bot/` | The example's shared commands, platform-agnostic, one `#[app]` per module (`Basics`, `Api`, `Interactive`, `Views`) nested in `Bot` (theme + events); each platform example nests `Bot` in its own App with its extra commands: `basics` (answers), `api` (REST, defer, files), `interactive` (buttons, modals), `views` (`#[view]` card, modal View, deferred re-render). `Vars` impls for `Worker` / `Host` behind `cloudflare` / `tokio` features. `tests/offline.rs` uses a `Platform` stand-in with a recording Transport; `tests/regression.rs` runs signed fixtures for every interaction shape; `tests/perf.rs` has allocation budgets. |
| `composure_example_cloudflare` | `examples/cloudflare/` | `CloudflareBot` on Workers (`fetch`, cron `scheduled`), wrangler config; `notes` (`/note`, Workers KV via `#[worker::send]` helpers); `sync-cloudflare` bin. |
| `composure_example_tokio` | `examples/tokio/` | `TokioBot` on tokio (`main`, `.env`), `Host<Server>` data; `server` (`/file` sandboxed `tokio::fs` + autocomplete, `/uptime`, `/primes` deferred with Background Work on `spawn_blocking`); `sync-tokio` bin; `tests/http.rs` replays the regression fixtures over HTTP and runs the server commands, `/primes`' deferred edits against a stand-in Discord (`HttpTransport::with_api_base`). |

Release profile is size-optimized (`opt-level = "z"`, LTO) because worker bundle size matters.

## Architecture

**Request flow (adapter):** the platform-independent steps live in `composure::adapter` (`handle_interaction_with_key`, `handle_event_with_key`, `send_with_files`, outcomes, header/var names), so Adapters only map outcomes to HTTP. Cloudflare: read body bytes → validate `X-Signature-Ed25519` / `X-Signature-Timestamp` against `DISCORD_PUBLIC_KEY` secret via an `auth::PublicKey` the adapter parses once per isolate (thread_local; `web-crypto`, on by default in the adapter, uses `crypto.subtle` on `wasm32-unknown-unknown`, else `ed25519-dalek`, which the workspace profile builds at `opt-level = 3`; `bench/` compares them on workerd, per verify and with the key cached) → deserialize `Interaction` → `composure::command::dispatch` to an `InteractionHandler<S>` (S = adapter State, `Worker` = `Env` + `Context` on Cloudflare) → return `InteractionResponse` as JSON (responses with files go through the callback endpoint, then 202). Ping → Pong in core. Handler `Err(HandlerError)` is logged and answered with `HandlerError::response()` (ephemeral).

**Defer:** dispatch wraps the Interaction in an `Arc` (`Ctx`/`Invocation` hold `&Arc<..>`, `Route::dispatch*` take `&Arc<..>`, `command::__unshare` hands it back by value to fallbacks), so `defer` shares it instead of copying. A State implementing `command::Platform` (Transport, bot token, `spawn`) enables `ctx.client()`, `ctx.bot()`, `ctx.defer(async |ctx: DeferredCtx| ..)`. `defer` spawns the Background Work (closure, then edit original with its output) through `Platform::spawn` (`wait_until` on Workers, `tokio::spawn` + `TaskTracker` on tokio) and returns type 5; dispatch signatures are unchanged (ADR 0006). Commands that use REST take `Ctx<impl Platform>` so tests can pass a stand-in State.

**Commands:** `#[command] fn x(ctx: Ctx, opts...)` keeps the fn and adds a same-named empty braced struct (type namespace only) implementing `CommandDefinition` (+ `SubcommandDefinition` for chat input) and `Command<S>`. Option params go through `OptionType` (extract + definition); compile-time rules are `const _: () = assert!(...)` blocks. `#[group]` routes via `Invocation::route`. **Apps (ADR 0013, `command/route.rs`, `crates/macros/src/app.rs`):** `#[app(handlers(..), theme = T, events = E)] struct Bot;` lists Routes in any mix; every attribute macro emits `RouteDefinition` (`const ROUTE: RouteInfo`, `THEME`, `definitions` for Sync) + `Route<S>` (static `dispatch`/`dispatch_autocomplete`/`dispatch_component`/`dispatch_modal`/`dispatch_event`, default `None`, leaves forward to `command::__dispatch_*`). An App chains its children, implements `InteractionHandler<S>`, `EventHandler<S>`, `CommandSetDefinition`, inherent `with_fallback`; Apps nest. `check_app` (const, walks the `RouteInfo` tree) rejects duplicate commands, Scope overflows, prefix collisions, unlisted placed sub-Views, two Apps with `events`. Futures are `Send` (ADR 0012): handler traits and `Transport` declare `fn .. -> impl Future<..> + Send` (impls write `async fn`), `Platform: Send + Sync`, `BackgroundWork` is `Send`, `Lazy` uses `OnceLock`; generic impls need `S: Sync` (the macros add it). Static dispatch, no boxing. Workers bridges JS with `worker::send::{SendWrapper, SendFuture}` (`Worker`, `FetchTransport`); handlers awaiting JS futures need `#[worker::send]`.

**Components/modals:** `#[component("prefix")] fn f(ctx: ComponentCtx, state..)` and `#[modal("prefix")] fn f(ctx: ModalCtx, form: F, state..)` (`F: #[derive(Modal)]`: each field is a Label around the input its attribute names — `input`/`select`/`user_select`/.../`radio`/`checkboxes`/`checkbox`/`file_upload` — and its type, via `command::ModalValue`, decides required-ness and how the value is read; `#[derive(Choice)]` enums supply options via `ModalChoice`) add a same-named struct implementing `ComponentHandler<S>` / `ModalHandler<S>` plus `f::custom_id(state..)` (and `f::show(state..)` for modals). State is encoded in the `custom_id` by `command::custom_id` (serde, `prefix:field:field`, `\` escapes, ADR 0007). Apps route them by prefix (`Route::dispatch_component/dispatch_modal`, also used by `WithFallback`). `Ctx<'a, S, D>` is generic over interaction data; `ComponentCtx`/`ModalCtx` are aliases. `#[autocomplete] fn f(ctx: Ctx, input: &str) -> Vec<..>` implements `Autocomplete<S>`; `#[option(.., autocomplete = f)]` sets `autocomplete: true` and generates `Command::autocomplete` routing the focused Option (Groups forward via `Invocation::route`); Apps expose it as `Route::dispatch_autocomplete`. Themes: `#[app(.., theme = THEME)]` sets `RouteDefinition::THEME` / `InteractionHandler::theme()` (nested App: its theme for `ctx.theme()` and filled first via `__themed`, outer ones fill the rest, errors use the root's); `ctx.theme()` (default palette when unset) gives `command::Theme` helpers (`embed`, `success`, ...); `dispatch` and deferred edits fill unset embed color/author/footer and top-level `Container` `accent_color` (`Unthemed` opts out), adapters render errors with `HandlerError::response_with(handler.theme())`; REST bodies are untouched (ADR 0009). Webhook Events: `types::{WebhookEventPayload, Event}` (unknown/mismatched → `Event::Unknown`, never an error), `command::EventHandler<S>` (per-event methods default to no-op, `handle` routes), `#[app(.., events = X)]` (at most one App in a tree; the root forwards to it; else ignored); the adapter serves `/events`, ACKs 204 and runs the handler via `spawn_event` (ADR 0008). Components: `types::Component` covers every type in the reference (layout: `ActionRow`, `Section`, `Container`, `Separator`, `Label`; content: `TextDisplay`, `Thumbnail`, `MediaGallery`, `FileComponent`, `UnfurledMedia`; inputs), each with an optional `id` (`Component::id`/`with_id`); unknown types deserialize to `Component::Unknown(Box<RawValue>)`, never an error. `IntoResponse for Container` / `InteractionResponse::respond_with_components` set `MessageFlags::IsComponentsV2` (no content/embeds alongside); message `components` fields are `Vec<Component>`. Regenerate trybuild expectations for new macro errors.

**Views (ADR 0011, `command/view.rs`, `crates/macros/src/view.rs`):** `rsx! { <Container>..</Container> }` is JSX (rstml, on syn 3 like the rest of `composure_macros`) expanded to `types::*` constructors + field assignments through `IntoAttr` (unknown attr = "no field" error); evaluates to `View` (V2 message) or `ModalView` (`<Modal>` root). `{expr}` children go through `IntoChildren`, text-only spots (`<Text>`, labels) through `IntoText`; adjacent text joins into one TextDisplay. `custom_id` errors ride inside the View and surface via `IntoResponse::try_into_response` (what `CommandOutput` calls; `Ephemeral`/`Update`/`Unthemed` forward it). `#[view] fn Card(props..) -> View` expands every `rsx!` in its body itself (so the bang macro alone never sees inline handlers), adds a same-named Props struct (+ typestate `Card::builder()` for `<Card a=1/>` markup), `IntoChildren`/`Rerender`/`IntoResponse` for it, `ViewDefinition::INFO` (prefix, handled, placed sub-Views) and `ViewHandler<S>`. Inline handlers `on:click:name=closure` become match arms on `prefix:name:captured-props`; captures are found syntactically (idents in the body, minus `name:` labels, plus `{name}` in format strings); output goes through autoref dispatch (`HandlerOutput`: `Rerender` → `Update`, else `CommandOutput`). Closures whose types use `<..>` must be braced in markup. A View listed in an App routes both components and modal submits; `check_app` const-checks placed sub-Views are listed.

**Tracing:** core `tracing` feature: `crate::trace` macros (`debug!`, `warn!`, `error!`, `info_span!`, `debug_span!`, `Instrument`, `Span`) forward to `tracing` or expand to nothing, so call sites have no `cfg`; spans `interaction` (dispatch), `background_work`/`event` (entered around `Platform::spawn`), `rest` (route template, never the URL/token); handler errors logged in-span by `HandlerError::trace` (adapter skips its own log with `tracing`). Core never installs a subscriber. Cloudflare adapter `tracing` feature: own logs via `log.rs` macros (console fallback without it), `tracing::init()` installs `ConsoleLayer` (JS objects to `console.*`, span fields merged, for Workers Logs); the example calls it in `#[event(start)]` (ADR 0010).

Adapters are the integration layer; new platforms go under `crates/adapters/<name>` and reuse core validation + types.

**Types (`crates/composure/src/types/`)** are one file per resource (interaction, message, channel, member, role, embed, component, ...), shared by Interactions, Responses and REST. All are glob re-exported, so everything is addressed as `composure::types::X`.

**REST (`crates/composure/src/rest/`, `rest` / `rest-*` / `tier1` / `full` features; one module + feature per resource, registered in `rest.rs`, `rest/spec.rs` `modules()`, and `Cargo.toml` tiers):** async clients over an Adapter-provided `Transport` (`send` + `sleep`; own `Request`/`RawResponse` types, core encodes JSON/multipart). `InteractionClient` (interaction token, no auth) and `BotClient` (bot token; resource Endpoints added as `impl BotClient<T>` blocks in per-feature modules). Each module declares its routes with `routes! { NAME = Method "/spec/{path}"; }` — paths copied from the spec so the spec check matches. Endpoints take borrowed body structs and return lazily-decoded `Response<T>` (`()` for 204). 429s are waited out via `Transport::sleep` up to `with_max_rate_limit_wait` (2s default, Sync uses 60s). Tests use `rest::testing::MockTransport`.

**Serde conventions:**
- `Interaction` uses a manual `Deserialize` that reads the body once into a private `Envelope` (common fields typed, `data` kept as `Box<RawValue>`), then parses `data` by `type` into `DataInteraction<T>` variants (`ApplicationCommandInteraction`, `MessageComponentInteraction`, `ModalSubmitInteraction` are type aliases). Options do the same via `RawOption`. This is the hot path: don't go through `serde_json::Value` or `#[serde(flatten)]` for type-discriminated unions; tagged unions (`Component`, definition models) read `type` from a `Box<RawValue>`, then parse the concrete type.
- Big fields handlers rarely read (`common.message`, `common.channel`) are `types::Lazy<T>`: raw JSON, parsed on first `get()` (`InteractionCommon::message()`/`channel()`). Resolved data is `SnowflakeMap` (a Vec, not a HashMap); `authorizing_integration_owners` is `IntegrationOwners`.
- In library code, parse with `from_slice` only (raw values via `raw.get().as_bytes()`), never `from_str`/`from_value`: a second reader duplicates every deserializer in the bundle (~180 KB). `examples/bot/tests/perf.rs` has per-path allocation budgets.
- `OptionType<'a>` / `CommandTarget<'a>` carry the Interaction's lifetime so `&str`, `&User`, `&Message`, ... parameters borrow instead of clone.
- `TypeField<const T: u8>` is a zero-sized marker that serializes as the constant and fails deserialization on mismatch — used as a `t` field to tag struct variants inside `#[serde(untagged)]`-style enums (see `crates/commands/src/command/model.rs`).
- Integer enums use `serde_repr`; flags use `bitflags`.
- `Snowflake` is a custom struct (decomposed timestamp/worker/process/increment) serialized as a string.

## Notes

- Docs: the user guide is `composure::guide` (`crates/composure/src/guide.rs`, one markdown chapter per module in `src/guide/`, doc-only, needs `macros` + `rest`; its examples are doctests run by `cargo test --workspace`). The Views chapter's tag reference lists every `rsx!` tag and attribute: update it (and the table on `rsx!` in `crates/macros/src/lib.rs`) when tags or component fields change. Every crate has `#![warn(missing_docs)]`; CI builds docs like docs.rs (`RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly doc --no-deps --all-features -p <crates>`).
- Glossary in `CONTEXT.md` (Command, App, Route, Option, Scope, Sync, ...); decisions in `docs/adr/`.
- All crates share version `0.1.0`; bump together (path deps also pin `version`).
- Regenerate trybuild expectations after changing macro errors: `TRYBUILD=overwrite cargo test -p composure_macros --test commands`, then review the `.stderr` diffs.

## Agent skills

### Issue tracker

GitHub Issues on BlueFrog130/composure-rs, via the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `CONTEXT.md` plus `docs/adr/`. See `docs/agents/domain.md`.
