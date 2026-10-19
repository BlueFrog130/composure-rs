# Composure example bot

A bot whose commands are each defined once with `#[command]` / `#[group]` / `#[derive(Choice)]` and collected with `commands!`. Nothing here depends on a platform. `example_bot! { pub struct MyBot = [extra, ...]; }` declares a Command Set with every command below plus a platform's own: [`examples/cloudflare`](../cloudflare) adds Workers KV notes, [`examples/tokio`](../tokio) adds local files, process data and blocking work. Each serves and syncs its own set (`Bot` here is the shared commands alone, for the tests). `/greet` reads its `GREETING` through the `Vars` trait, implemented for each Adapter's State behind the `cloudflare` / `tokio` features.

[`src/basics.rs`](src/basics.rs): answering directly

| Command | Shows |
|---|---|
| `/hello` | `Ctx` (invoking user/member), user-install contexts |
| `/echo` | Options with constraints, optional Option, borrowed `&str` |
| `/ice-cream` | `#[derive(Choice)]` and its `name()`, `name` override, `Embed` response |
| `/fruit` | `#[autocomplete]` handler attached with `#[option(.., autocomplete = fruit_names)]` |
| `/greet` | `Ctx<impl Vars>`: a bounded State, `Worker` on Workers, `Host` on tokio, a stand-in in tests |
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

[`src/interactive.rs`](src/interactive.rs): `#[component]` / `#[modal]` handlers, listed in `commands!` under `components` / `modals`

| Command | Shows |
|---|---|
| `/counter` | `#[component("counter")]` with the count as `custom_id` state (`counter_click::custom_id(n)`); `Update(..)` edits the clicked message |
| `/roll` | `ctx.defer_update(..)`: acknowledge without a loading message, edit the message from Background Work |
| `/colors` | A string select; `ComponentCtx::values()` |
| `/feedback` | `#[derive(Modal)]` form (text inputs, a Choice-backed radio group, file upload, checkbox) opened with `on_feedback::show()`, submitted to `#[modal("feedback")]` typed |

[`src/events.rs`](src/events.rs): Webhook Events (`commands! { ..; events = Events; }`), served on `/events`

| Event | Shows |
|---|---|
| `APPLICATION_AUTHORIZED` | A user install gets a welcome DM, sent after the 204 with `EventCtx::bot()` |

## Test

`cargo test -p composure_example_bot` sends signed interactions through `composure::adapter::handle_interaction`, the pipeline both Adapters run. The stand-in State in `tests/offline.rs` implements `Platform` with a recording Transport, so REST calls are asserted without Discord, and runs Background Work on demand. `tests/regression.rs` covers every interaction shape with signed fixtures (also replayed over HTTP by `examples/tokio`), and `tests/perf.rs` fails if a path allocates more than its budget (`cargo test -p composure_example_bot --release --test perf -- --ignored --nocapture` for timings).
