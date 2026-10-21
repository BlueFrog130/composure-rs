<div align="center">

# Composure

**Discord bots that live on the edge.**

Slash commands, buttons, modals and rich Components V2 layouts, written once in Rust and served
from Cloudflare Workers or any server. No gateway, no idle process, no bill while nobody is
talking to your bot.

[![Crates.io](https://img.shields.io/crates/v/composure.svg?logo=rust)](https://crates.io/crates/composure)
[![Docs.rs](https://img.shields.io/docsrs/composure?logo=docs.rs)](https://docs.rs/composure)
[![CI](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml)
[![Downloads](https://img.shields.io/crates/d/composure.svg)](https://crates.io/crates/composure)
[![License](https://img.shields.io/crates/l/composure.svg)](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.88+-orange.svg?logo=rust)](https://blog.rust-lang.org/)
<br>
[![Discord API](https://img.shields.io/badge/Discord_API-v10-5865F2?logo=discord&logoColor=white)](https://discord.com/developers/docs/interactions/overview)
[![Cloudflare Workers](https://img.shields.io/badge/runs_on-Cloudflare_Workers-F38020?logo=cloudflareworkers&logoColor=white)](https://workers.cloudflare.com/)
[![tokio](https://img.shields.io/badge/runs_on-tokio-463F3A?logo=rust&logoColor=white)](https://tokio.rs/)
[![WASM](https://img.shields.io/badge/target-wasm32-654FF0?logo=webassembly&logoColor=white)](https://webassembly.org/)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)

[Guide](https://bluefrog130.github.io/composure-rs/) ·
[API docs](https://docs.rs/composure) ·
[Examples](https://github.com/BlueFrog130/composure-rs/tree/main/examples) ·
[Crates](#crates)

</div>

---

```rust
use composure::{app, command, types::User};

/// Give someone a cookie
#[command]
fn cookie(
    #[option("Who gets it")] user: &User,
    #[option("How many", min = 1, max = 12)] count: Option<u8>,
) -> String {
    format!("{} gets {} 🍪", user.display_name(), count.unwrap_or(1))
}

#[app(handlers(cookie))]
pub struct Bot;
```

That function **is** the command. Its parameters become Discord Options (with `min`/`max`
enforced by Discord and checked against `u8` at compile time), its doc comment the description,
its return value the reply. Hand `Bot` to an Adapter to serve it, and to `composure_sync` to
register it.

## Why Composure?

Discord's future is [Interactions](https://discord.com/developers/docs/interactions/overview):
commands, buttons, selects and modals that Discord delivers to your bot as signed HTTP requests.
Composure is built for that model and nothing else.

- **☁️ Serverless by design.** No WebSocket to keep alive, no process to babysit. Deploy to
  Cloudflare Workers and pay per request, or run it on any box with tokio.
- **✍️ Write it once.** One function per command: the definition Discord stores and the handler
  that answers come from the same code, so they can never drift apart.
- **🦀 Typed end to end.** Options arrive as `&str`, `u8`, `&User`, `Role` or your own enums.
  Duplicate commands, too many Options, out-of-range bounds and bad `custom_id` prefixes fail
  to compile, not in production.
- **🧩 Rich UIs in markup.** Build Components V2 messages and modals with JSX-like `rsx!`,
  with click handlers right next to the buttons they belong to.
- **💾 Stateless state.** Button and modal state rides in the `custom_id`, encoded for you. No
  database for a counter.
- **⚡ Tiny and fast.** A few microseconds and a few dozen allocations per Interaction; the full
  example bot is about 344 KB of gzipped WASM. The core has no async runtime and no HTTP stack.
- **🔋 Batteries included.** A REST client for every Discord endpoint, deferred responses with
  background work, follow-ups, file uploads, Webhook Events, themes, `tracing`, and a command
  sync CLI with CI drift checks.

| | Gateway bot | Composure |
|---|---|---|
| Connection | A WebSocket, always open | None: Discord calls you |
| Hosting | A server running 24/7 | Cloudflare Workers, a container, any server |
| Scales to zero | No | Yes |
| Commands defined | Often twice (registration + handler) | Once, as a function |
| Reads every message | Possible (with intents) | No: Interactions and Webhook Events only |

If your bot needs to watch every message or presence update, use a gateway library. If it
responds to people, Composure does it with a fraction of the infrastructure.

## A tour

### Rich messages with `rsx!` and `#[view]`

```rust
use composure::{rsx, view, command::View};

/// A card that counts its likes. The count lives in the button's custom_id.
#[view]
pub fn Card(title: String, likes: u32) -> View {
    rsx! {
        <Container>
            "## " {title.as_str()}
            <Text>"-# " {likes} " likes"</Text>
            <Row>
                <Button primary on:click:like=move |_| Card { likes: likes + 1, title }>
                    "Like"
                </Button>
                <Button url="https://discord.com/developers/docs">"Docs"</Button>
            </Row>
        </Container>
    }
}
```

Clicking **Like** re-renders the message with new Props. Every Discord component has a tag,
and every tag's attributes are [documented](https://bluefrog130.github.io/composure-rs/views.html#tag-reference).

### Forms that arrive typed

```rust
use composure::{Choice, Modal, modal, command::Ephemeral, types::Attachment};

#[derive(Choice, Debug)]
pub enum Kind { Bug, Idea }

#[derive(Modal)]
#[modal(title = "Feedback")]
pub struct Feedback {
    #[input(max_len = 100)]
    topic: String,
    #[radio]
    kind: Kind,
    #[file_upload(max = 3, file_types("image"))]
    screenshots: Vec<Attachment>,
}

#[modal("feedback")]
fn on_feedback(form: Feedback) -> Ephemeral<String> {
    Ephemeral(format!("Thanks! {:?}: {} ({} screenshots)", form.kind, form.topic, form.screenshots.len()))
}
```

### Slow work, answered in time

```rust
use std::time::Duration;
use composure::{command, command::{Ctx, Deferred, HandlerError, Platform}};

/// Crunch some numbers
#[command]
fn crunch(ctx: Ctx<impl Platform>) -> Deferred {
    ctx.defer(async |ctx| {
        ctx.sleep(Duration::from_secs(5)).await; // well past Discord's 3 seconds
        let me = ctx.bot()?.get_current_user().await?.model()?;
        Ok::<_, HandlerError>(format!("{} finished crunching", me.username))
    })
}
```

### Serve it anywhere

```rust
// Cloudflare Workers
#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> worker::Result<Response> {
    CloudflareInteractionBot::new(req, env, ctx).with_handler(Bot).process().await
}

// Any server
#[tokio::main]
async fn main() -> Result<(), composure_adapter_tokio::Error> {
    TokioInteractionBot::from_env()?.with_handler(Bot).serve().await
}
```

### Ship it

```sh
cargo run --bin sync             # show what changes on Discord, apply after confirming
cargo run --bin sync -- --check  # in CI: fail when Discord is out of date
```

## Quick start

```sh
cargo add composure --features macros,tier1
cargo add composure_adapter_cloudflare   # or composure_adapter_tokio
cargo add composure_sync                 # in a host-only `sync` binary
```

Or start from a template, with a Command, a Sync binary, a test, and the platform's
configuration (a `Dockerfile` for tokio):

```sh
cargo generate --git https://github.com/BlueFrog130/composure-rs templates/cloudflare --name my-bot
cargo generate --git https://github.com/BlueFrog130/composure-rs templates/tokio --name my-bot
```

Then follow [Getting started](https://bluefrog130.github.io/composure-rs/getting_started.html):
it takes you from an empty crate to a deployed bot.

## Crates

| Crate | | |
|---|---|---|
| [`composure`](https://crates.io/crates/composure) | [![composure version](https://img.shields.io/crates/v/composure.svg)](https://crates.io/crates/composure) | The core: types, the handler runtime, the REST client, and the macros (`macros` feature) |
| [`composure_adapter_cloudflare`](https://crates.io/crates/composure_adapter_cloudflare) | [![composure_adapter_cloudflare version](https://img.shields.io/crates/v/composure_adapter_cloudflare.svg)](https://crates.io/crates/composure_adapter_cloudflare) | Serve on Cloudflare Workers |
| [`composure_adapter_tokio`](https://crates.io/crates/composure_adapter_tokio) | [![composure_adapter_tokio version](https://img.shields.io/crates/v/composure_adapter_tokio.svg)](https://crates.io/crates/composure_adapter_tokio) | Serve from a server on tokio and hyper |
| [`composure_sync`](https://crates.io/crates/composure_sync) | [![composure_sync version](https://img.shields.io/crates/v/composure_sync.svg)](https://crates.io/crates/composure_sync) | Register your commands with Discord at deploy time |
| [`composure_macros`](https://crates.io/crates/composure_macros) | [![composure_macros version](https://img.shields.io/crates/v/composure_macros.svg)](https://crates.io/crates/composure_macros) | The macros, re-exported by `composure` |
| [`composure_commands`](https://crates.io/crates/composure_commands) | [![composure_commands version](https://img.shields.io/crates/v/composure_commands.svg)](https://crates.io/crates/composure_commands) | A builder for command definitions, without the macros |

Everything optional is behind a feature flag (REST endpoints per resource, `tracing`, Web
Crypto signature checks), so a Worker only ships what it uses. See
[feature flags](https://docs.rs/composure/latest/composure/#feature-flags).

## Learn more

- **[The guide](https://bluefrog130.github.io/composure-rs/)**: commands,
  responses, components and modals, Views, themes, REST and deferring, Webhook Events,
  platforms, testing. Every example in it is compiled and tested.
- **[Examples](https://github.com/BlueFrog130/composure-rs/tree/main/examples)**: one bot using
  every feature, served [on Workers](https://github.com/BlueFrog130/composure-rs/tree/main/examples/cloudflare)
  and [on tokio](https://github.com/BlueFrog130/composure-rs/tree/main/examples/tokio).
- **[The Composure bot](https://github.com/BlueFrog130/composure-rs/tree/main/bots/composure)**:
  Composure's own bot on Workers, a real deployment. Add it to get release announcements in
  your server (`/notify setup`), read the changelog (`/changelog`) and search these docs
  (`/docs`); `/invite` in any server it's in gives the install links.
- **[Design decisions](https://github.com/BlueFrog130/composure-rs/tree/main/docs/adr)** and the
  [glossary](https://github.com/BlueFrog130/composure-rs/blob/main/CONTEXT.md).

## Status

Composure is young and pre-1.0: the API may change between minor versions. It tracks Discord's
v10 API and its latest components (Components V2, labels, radio groups, checkboxes, file
uploads). Issues and pull requests are welcome.

## Contributing

See [CONTRIBUTING.md](https://github.com/BlueFrog130/composure-rs/blob/main/CONTRIBUTING.md) for
the development setup and how releases work: each pull request adds a change file
(`knope document-change`), and merging the generated release PR publishes every crate. Every
version is in the [changelog](https://github.com/BlueFrog130/composure-rs/blob/main/CHANGELOG.md).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE).
