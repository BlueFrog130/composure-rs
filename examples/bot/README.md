<img src="../../assets/logo.svg" alt="Composure logo" width="64" height="64" align="right">

# Composure example bot

[![CI](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml)
[![Guide](https://img.shields.io/badge/guide-mdBook-blue?logo=mdbook)](https://bluefrog130.github.io/composure-rs/)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE)

A bot whose commands are each defined once with `#[command]` / `#[group]` / `#[derive(Choice)]` and collected with `#[app]`: each module lists its own in an App (`Basics`, `Api`, `Interactive`, `Views`), and `Bot` lists those with the Webhook Events and its error style (`errors = style::render_error`); `style` holds its colors and a branded embed. Nothing here depends on a platform. Each example lists `Bot` in its own App next to its platform's commands (`#[app(handlers(Bot, Note))]`): [`examples/cloudflare`](../cloudflare) adds Workers KV notes, [`examples/tokio`](../tokio) adds local files, process data and blocking work, and each serves and syncs that App (the tests use `Bot` alone). `/greet` reads its greeting from the bot's Bot Data (`Config`) through `HasData`: Workers build it from the `GREETING` var with `with_data`, tokio from the environment, inside its own `Server` data (`AsRef<Config>`).

[`src/basics.rs`](src/basics.rs): answering directly

| Command | Shows |
|---|---|
| `/hello` | `Ctx` (invoking user/member), user-install contexts |
| `/echo` | Options with constraints, optional Option, borrowed `&str` |
| `/ice-cream` | `#[derive(Choice)]` and its `name()`, `name` override, `Embed` response |
| `/fruit` | `#[autocomplete]` handler attached with `#[option(.., autocomplete = fruit_names)]` |
| `/greet` | `Ctx<impl HasData<Data: AsRef<Config>>>`: Bot Data, `Worker<Config>` on Workers, `Host<Server>` on tokio, a stand-in in tests |
| `/mod ban`, `/mod kick` | `#[group]` with `scope = "dev"` and permissions, `Ephemeral` replies, `HandlerError::user` |
| Avatar (user), Word count (message) | Context-menu commands |

[`src/api.rs`](src/api.rs): Discord's REST API, through `Ctx<impl Platform>`

| Command | Shows |
|---|---|
| `/whoami` | `ctx.bot()` inline (bot token): Get Current User, `Response::model()` |
| `/remind` | `Ephemeral(ctx.defer(..))`, `DeferredCtx::sleep`, a follow-up message, result edits the original |
| `/countdown` | Editing the original response repeatedly from Background Work |
| `/announce` | Create Message as the bot (dev guild, `ManageMessages`); the response body is never parsed |
| Thumbs up (message) | Create Reaction on the target message |
| `/dm-me` | Create DM + Create Message; a REST failure turned into a `HandlerError::user` |
| `/report` | Bot Client in Background Work (Get Commands), a CSV file uploaded with the edit (multipart) |
| `/command-info` | Async autocomplete with `Ctx<impl Platform>`: suggestions fetched with the bot token |
| `/export` | A file in the initial response (the Adapter answers through the callback endpoint) |

[`src/interactive.rs`](src/interactive.rs): `#[component]` / `#[modal]` handlers, listed in the module's App next to the Commands that send them

| Command | Shows |
|---|---|
| `/counter` | `#[component("counter")]` with the count as `custom_id` state (`counter_click::custom_id(n)`); `Update(..)` edits the clicked message |
| `/roll` | `ctx.defer_update(..)`: acknowledge without a loading message, edit the message from Background Work |
| `/colors` | A string select; `ComponentCtx::values()` |
| `/feedback` | `#[derive(Modal)]` form (text inputs, a Choice-backed radio group, file upload, checkbox) opened with `on_feedback::show()`, submitted to `#[modal("feedback")]` typed |

[`src/events.rs`](src/events.rs): Webhook Events (`#[app(.., events = Events)]` on `Bot`), served on `/events`

| Event | Shows |
|---|---|
| `APPLICATION_AUTHORIZED` | A user install gets a welcome DM, sent after the 204 with `EventCtx::bot()` |

## Test

`cargo test -p composure_example_bot` sends signed interactions through `composure::adapter::handle_interaction`, the pipeline both Adapters run. The stand-in State in `tests/offline.rs` implements `Platform` with a recording Transport, so REST calls are asserted without Discord, and runs Background Work on demand. `tests/regression.rs` covers every interaction shape with signed fixtures (also replayed over HTTP by `examples/tokio`), and `tests/perf.rs` fails if a path allocates more than its budget (`cargo test -p composure_example_bot --release --test perf -- --ignored --nocapture` for timings).
