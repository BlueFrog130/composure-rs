//! Composure is a framework for Discord bots that only answer
//! [Interactions](https://discord.com/developers/docs/interactions/receiving-and-responding)
//! over HTTP: no gateway connection, so a bot runs on the edge (Cloudflare Workers, compiled to
//! WASM) or on any hosted server (tokio), and scales to zero between requests.
//!
//! ```
//! # #[cfg(feature = "macros")] mod example {
//! use composure::{app, command};
//!
//! /// Repeat some text
//! #[command]
//! fn echo(
//!     #[option("What to say", max_len = 200)] text: &str,
//!     #[option("How many times", min = 1, max = 5)] times: Option<u8>,
//! ) -> String {
//!     text.repeat(times.unwrap_or(1).into())
//! }
//!
//! /// Everything the bot serves
//! #[app(handlers(echo))]
//! pub struct Bot;
//! # }
//! # fn main() {}
//! ```
//!
//! Each Command is written once, as a function: its parameters become the Command's Options, its
//! doc comment the description, its return value the answer. `Bot` is then handed to an Adapter
//! to serve Interactions, and to `composure_sync` at deploy time to register the Commands with
//! Discord.
//!
//! # Where to start
//!
//! The [guide](https://bluefrog130.github.io/composure-rs/) walks through everything a bot
//! does, chapter by chapter:
//!
//! | Chapter | |
//! |---|---|
//! | [Getting started](https://bluefrog130.github.io/composure-rs/getting_started.html) | Crates, features, a first bot, serving it, syncing it |
//! | [Commands](https://bluefrog130.github.io/composure-rs/commands.html) | `#[command]`, Options, Choices, Groups, context menus, autocomplete, Scopes |
//! | [Responses](https://bluefrog130.github.io/composure-rs/responses.html) | What a handler can return; ephemeral messages, embeds, files, errors |
//! | [Components and modals](https://bluefrog130.github.io/composure-rs/components.html) | Buttons, selects and forms with `#[component]`, `#[modal]`, `#[derive(Modal)]` |
//! | [Views](https://bluefrog130.github.io/composure-rs/views.html) | `rsx!` markup and `#[view]`: every tag and attribute |
//! | [Themes](https://bluefrog130.github.io/composure-rs/theming.html) | Consistent colors and branding on embeds and containers |
//! | [REST and deferring](https://bluefrog130.github.io/composure-rs/deferring.html) | `ctx.bot()`, follow-ups, Deferred Responses, Background Work |
//! | [Webhook Events](https://bluefrog130.github.io/composure-rs/events.html) | App installs, entitlements, lobby messages |
//! | [Platforms](https://bluefrog130.github.io/composure-rs/platforms.html) | Adapters, State, platform-specific Commands, logging |
//! | [Testing](https://bluefrog130.github.io/composure-rs/testing.html) | Running handlers without Discord |
//!
//! # Crates
//!
//! | Crate | |
//! |---|---|
//! | `composure` (this one) | Types, request validation, the handler runtime, the REST client, and (with `macros`) the attribute macros |
//! | [`composure_adapter_cloudflare`](https://docs.rs/composure_adapter_cloudflare) | Serves a bot on Cloudflare Workers |
//! | [`composure_adapter_tokio`](https://docs.rs/composure_adapter_tokio) | Serves a bot from a hosted server on tokio |
//! | [`composure_sync`](https://docs.rs/composure_sync) | Registers an App's Commands with Discord at deploy time |
//! | [`composure_macros`](https://docs.rs/composure_macros) | The macros; use them through `composure`'s `macros` feature |
//! | [`composure_commands`](https://docs.rs/composure_commands) | A builder for Command Definitions, for bots not using `#[command]` |
//!
//! # Modules
//!
//! - [`command`](mod@command): the handler runtime: [`Ctx`], responses ([`Ephemeral`](command::Ephemeral),
//!   [`Update`](command::Update), ...), [`Theme`](command::Theme), [`View`](command::View),
//!   errors, and the traits the macros implement
//! - [`types`]: Discord's objects (Interactions, messages, embeds, components, users, ...),
//!   shared by Interactions, responses and REST
//! - [`definition`]: Command Definitions, as Discord stores them
//! - [`rest`]: the REST client over an Adapter's [`Transport`](rest::Transport)
//! - [`adapter`]: the request pipeline every Adapter shares (validate, parse, dispatch)
//! - [`auth`]: Ed25519 request signature validation
//! - [`credentials`]: [`Credentials`], what identifies the application, however the bot loads them
//! - `testing` (`testing` feature): Interaction builders, a stand-in Platform and Transport,
//!   signing keys, for testing a bot without Discord
//!
//! # Feature flags
//!
//! No feature is on by default: the core stays small for WASM bundles.
//!
//! | Feature | |
//! |---|---|
//! | `macros` | `#[app]`, `#[command]`, `#[group]`, `#[component]`, `#[modal]`, `#[autocomplete]`, `#[view]`, `rsx!`, `#[derive(Choice)]`, `#[derive(Modal)]` |
//! | `rest` | The REST client ([`rest`]) with the Interaction Endpoints, [`Platform`](command::Platform), `ctx.defer(..)`, and the [`adapter`] pipeline |
//! | `rest-commands`, `rest-messages`, `rest-users`, `rest-channels`, `rest-guilds`, `rest-members`, `rest-roles` | One group of Endpoints each on [`BotClient`](rest::BotClient) |
//! | `rest-bans`, `rest-threads`, `rest-invites`, `rest-emojis`, `rest-stickers`, `rest-scheduled-events`, `rest-automod`, `rest-audit-log`, `rest-webhooks`, `rest-monetization`, `rest-polls`, `rest-soundboard`, `rest-application` | The rest of Discord's Endpoints, one group each |
//! | `tier1` | `rest` and the Endpoints most bots need (commands, messages, users, channels, guilds, members, roles) |
//! | `full` | Every Endpoint |
//! | `tracing` | Spans and events through [`tracing`](https://docs.rs/tracing); see [Platforms](https://bluefrog130.github.io/composure-rs/platforms.html#logging) |
//! | `web-crypto` | Validate signatures with the host's Web Crypto on `wasm32-unknown-unknown` instead of compiling `ed25519-dalek` in |
//! | `reqwest` | A blocking [`ReqwestTransport`](rest::ReqwestTransport) for deploy-time tools (not for WASM) |
//! | `testing` | `rest` and `composure::testing`, for a bot's tests: Interaction builders, `TestPlatform`, `MockTransport`, `TestKey`; see [Testing](https://bluefrog130.github.io/composure-rs/testing.html) |

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

#[macro_use]
mod trace;

#[cfg(feature = "rest")]
pub mod adapter;
pub mod auth;
pub mod command;
pub mod credentials;
pub mod definition;
mod outline;
// The guide (`book/`) as doctests
#[cfg(all(feature = "macros", feature = "testing", doctest))]
mod guide;
#[cfg(feature = "rest")]
pub mod rest;
#[cfg(all(feature = "rest", any(test, feature = "testing")))]
#[cfg_attr(docsrs, doc(cfg(feature = "testing")))]
pub mod testing;
pub mod types;

pub use command::Ctx;
pub use credentials::{Credentials, CredentialsError};

/// Collects Routes into an App, the handler an Adapter serves. See
/// [Getting started](https://bluefrog130.github.io/composure-rs/getting_started.html) and [Platforms](https://bluefrog130.github.io/composure-rs/platforms.html).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::app;

/// Defines a Command from a function. See [Commands](https://bluefrog130.github.io/composure-rs/commands.html).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::command;

/// Groups Commands as subcommands. See [Commands](https://bluefrog130.github.io/composure-rs/commands.html#groups).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::group;

/// Answers a component by `custom_id` prefix. See
/// [Components and modals](https://bluefrog130.github.io/composure-rs/components.html).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::component;

/// Answers a modal submit by `custom_id` prefix. See
/// [Components and modals](https://bluefrog130.github.io/composure-rs/components.html#modals).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::modal;

/// Suggests values for an Option while the user types. See
/// [Commands](https://bluefrog130.github.io/composure-rs/commands.html#autocomplete).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::autocomplete;

/// An enum of fixed Choices, for Options, selects and radio groups. See
/// [Commands](https://bluefrog130.github.io/composure-rs/commands.html#choices).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::Choice;

/// A form shown as a modal. See [Components and modals](https://bluefrog130.github.io/composure-rs/components.html#forms).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::Modal;

/// Markup for message and modal layouts. Every tag and attribute is listed in
/// [Views](https://bluefrog130.github.io/composure-rs/views.html#tag-reference).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::rsx;

/// A layout rendered from Props, with inline handlers. See [Views](https://bluefrog130.github.io/composure-rs/views.html).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::view;

/// Something Discord renders as a mention in message content: `<@id>` for users, `<#id>` for
/// channels, `<@&id>` for roles.
pub trait Mentionable {
    /// The mention markup, e.g. `<@80351110224678912>`
    fn to_mention(&self) -> String;
}
