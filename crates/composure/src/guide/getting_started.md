From an empty crate to a bot answering `/hello`: define Commands, collect them in an App, serve
the App with an Adapter, and sync its Commands with Discord.

# How a bot works

Discord sends every Interaction (a slash Command, a button click, a modal submit, an
autocomplete request) as a signed HTTP `POST` to the bot's **Interactions Endpoint URL**. The
bot verifies the signature, answers within 3 seconds, and can keep working afterwards through
the REST API. There is no gateway connection to keep alive, so the bot can run where requests
are cheap and start fast: a serverless platform like Cloudflare Workers, or any server.

Composure splits this in three:

- **Your bot**, written against `composure`: Commands and handlers collected into an
  [`#[app]`](crate::app). Platform-agnostic.
- **An Adapter**, which serves the App on one platform: it reads the request, validates the
  signature, dispatches to the App, and writes the response.
  [`composure_adapter_cloudflare`](https://docs.rs/composure_adapter_cloudflare) and
  [`composure_adapter_tokio`](https://docs.rs/composure_adapter_tokio) exist today.
- **Sync** ([`composure_sync`](https://docs.rs/composure_sync)), run at deploy time, which
  registers the App's Command Definitions with Discord.

# Setting up

Create an application in the [Developer Portal](https://discord.com/developers/applications)
and note its **application id**, **public key** and (under *Bot*) **token**.

Add the crates. On Cloudflare Workers (a `cdylib` built with `worker-build`):

```toml
[dependencies]
composure = { version = "0.1", features = ["macros", "tier1"] }
composure_adapter_cloudflare = "0.1"
worker = "0.8"
```

On a server:

```toml
[dependencies]
composure = { version = "0.1", features = ["macros", "tier1"] }
composure_adapter_tokio = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

`macros` brings the attribute macros; `tier1` the REST client with the Endpoints most bots need.
See [feature flags](crate#feature-flags) for the rest; leaving REST out keeps a Worker's bundle
smaller.

# A first Command

```
use composure::{app, command, command::Ctx};

/// Say hello
#[command]
fn hello(ctx: Ctx) -> String {
    let name = ctx.user().map_or("stranger", |user| user.display_name());
    format!("Hello, {name}!")
}

/// Everything the bot serves
#[app(handlers(hello))]
pub struct Bot;
# fn main() {}
```

The function is the handler; the doc comment, the name and the parameters are the
definition Discord shows. [Commands](crate::guide::commands) covers Options, Groups and more.

# Apps

[`#[app(handlers(..))]`](crate::app) on a unit struct makes an App: the handler an Adapter
serves and the set of Commands Sync registers. It lists, in any order and mix:

- Commands ([`#[command]`](macro@crate::command)) and Groups ([`#[group]`](crate::group))
- component and modal handlers ([`#[component]`](crate::component), [`#[modal]`](crate::modal))
- Views with inline handlers ([`#[view]`](crate::view))
- other Apps

Apps nest, so each module of a bot can declare its own App next to its handlers, and the root
App lists the modules':

```
mod moderation {
    use composure::{app, command, types::User};

    /// Kick a member
    #[command]
    fn kick(#[option("Who")] user: &User) -> String {
        format!("Kicked {}", user.display_name())
    }

    #[app(handlers(kick))]
    pub struct Moderation;
}

mod fun {
    use composure::{app, command};

    /// Flip a coin
    #[command]
    fn flip() -> &'static str {
        "Heads"
    }

    #[app(handlers(flip))]
    pub struct Fun;
}

use composure::app;

#[app(handlers(moderation::Moderation, fun::Fun), theme = THEME)]
pub struct Bot;

const THEME: composure::command::Theme = composure::command::Theme::new().with_primary(0xf5c2e7);
# fn main() {}
```

`#[app]` also takes `theme = ..` ([Themes](crate::guide::theming)) and `events = ..`
([Webhook Events](crate::guide::events)).

Mistakes that would only show up at runtime fail to compile instead: two Commands with the same
name and type, two handlers with the same `custom_id` prefix, more Commands in a
[Scope](crate::guide::commands#scopes) than Discord allows, a sub-View with handlers that no App
lists, two Apps setting `events`.

An App implements [`InteractionHandler`](crate::command::InteractionHandler) (what Adapters
serve), [`EventHandler`](crate::command::EventHandler), and
[`CommandSetDefinition`](crate::command::CommandSetDefinition) (what Sync registers).
Interactions nothing in the App handles get a default answer, or go to a fallback set with
`Bot.with_fallback(handler)` ([Fallback](crate::guide::components#fallback)).

# Serving it

## Cloudflare Workers

```rust,ignore
use composure_adapter_cloudflare::CloudflareInteractionBot;
use worker::{Context, Env, Request, Response, event};

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> worker::Result<Response> {
    CloudflareInteractionBot::new(req, env, ctx)
        .with_handler(Bot)
        .process()
        .await
}
```

Set the secrets with `wrangler secret put DISCORD_PUBLIC_KEY` (required) and `DISCORD_TOKEN`
(for the bot client), or load the [Credentials](crate::guide::platforms#credentials) another
way with `.with_credentials(..)`; deploy, and put `https://<worker>.workers.dev/` in the Developer Portal as
the Interactions Endpoint URL.

## A server, with tokio

```rust,ignore
use composure_adapter_tokio::TokioInteractionBot;

#[tokio::main]
async fn main() -> Result<(), composure_adapter_tokio::Error> {
    TokioInteractionBot::from_env()? // DISCORD_PUBLIC_KEY, DISCORD_TOKEN, DISCORD_APPLICATION_ID, PORT
        .with_handler(Bot)
        .serve() // 0.0.0.0:$PORT (8080), until Ctrl-C / SIGTERM
        .await
}
```

`from_env` is one way to get the [Credentials](crate::guide::platforms#credentials):
`TokioInteractionBot::new(credentials)` takes them from anywhere, the `config` crate included.
It speaks plain HTTP: put it behind a TLS-terminating proxy or tunnel, and use that HTTPS URL as
the Interactions Endpoint URL.

Discord checks the URL when you save it (a signed Ping, which the Adapter answers), so the bot
must be running first. See [Platforms](crate::guide::platforms) for State, Bot Data (your own
configuration), platform-specific Commands and logging.

# Syncing Commands

Discord only shows Commands it has stored. [`composure_sync`](https://docs.rs/composure_sync)
makes the stored ones match the App: from a host-only binary in your crate,

```rust,ignore
// src/bin/sync.rs
fn main() -> std::process::ExitCode {
    composure_sync::sync_from_env(&my_bot::Bot)
}
```

```sh
DISCORD_TOKEN=.. DISCORD_APPLICATION_ID=.. cargo run --bin sync
```

It prints a plan (Commands to create, update and delete, per target), and applies it after
confirmation; `--dry-run`, `--check` (CI drift check), `--yes` (CI deploys) and `--guild <id>`
(send everything to one test server) adjust that.

Commands with a [Scope](crate::guide::commands#scopes) go to the guilds listed for it:
`COMPOSURE_SCOPE_DEV=123,456` for `scope = "dev"`. Sync again whenever a Command's definition
changes; handlers can change freely without it.

On Workers, keep Sync out of the Worker build: put it in a `[[bin]]` that only builds on the
host, as the repository's `examples/cloudflare` does.

# Next steps

- [Commands](crate::guide::commands): Options, Choices, autocomplete, Groups, context menus
- [Responses](crate::guide::responses): embeds, ephemeral messages, files, errors
- [Views](crate::guide::views): rich Components V2 messages and modals in markup
- [REST and deferring](crate::guide::deferring): calling Discord, answering slowly
- The repository's `examples/` hold a complete bot served both ways.
