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
//! The [`guide`] walks through everything a bot does, chapter by chapter:
//!
//! | Chapter | |
//! |---|---|
//! | [Getting started](guide::getting_started) | Crates, features, a first bot, serving it, syncing it |
//! | [Commands](guide::commands) | `#[command]`, Options, Choices, Groups, context menus, autocomplete, Scopes |
//! | [Responses](guide::responses) | What a handler can return; ephemeral messages, embeds, files, errors |
//! | [Components and modals](guide::components) | Buttons, selects and forms with `#[component]`, `#[modal]`, `#[derive(Modal)]` |
//! | [Views](guide::views) | `rsx!` markup and `#[view]`: every tag and attribute |
//! | [Themes](guide::theming) | Consistent colors and branding on embeds and containers |
//! | [REST and deferring](guide::deferring) | `ctx.bot()`, follow-ups, Deferred Responses, Background Work |
//! | [Webhook Events](guide::events) | App installs, entitlements, lobby messages |
//! | [Platforms](guide::platforms) | Adapters, State, platform-specific Commands, logging |
//! | [Testing](guide::testing) | Running handlers without Discord |
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
//! | `tracing` | Spans and events through [`tracing`](https://docs.rs/tracing); see [Platforms](guide::platforms#logging) |
//! | `web-crypto` | Validate signatures with the host's Web Crypto on `wasm32-unknown-unknown` instead of compiling `ed25519-dalek` in |
//! | `reqwest` | A blocking [`ReqwestTransport`](rest::ReqwestTransport) for deploy-time tools (not for WASM) |

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
#[cfg(all(feature = "macros", feature = "rest", any(doc, doctest)))]
pub mod guide;
#[cfg(feature = "rest")]
pub mod rest;
pub mod types;

pub use command::Ctx;
pub use credentials::{Credentials, CredentialsError};

/// Collects Routes into an App, the handler an Adapter serves. See
/// [Getting started](crate::guide::getting_started) and [Platforms](crate::guide::platforms).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::app;

/// Defines a Command from a function. See [Commands](crate::guide::commands).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::command;

/// Groups Commands as subcommands. See [Commands](crate::guide::commands#groups).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::group;

/// Answers a component by `custom_id` prefix. See
/// [Components and modals](crate::guide::components).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::component;

/// Answers a modal submit by `custom_id` prefix. See
/// [Components and modals](crate::guide::components#modals).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::modal;

/// Suggests values for an Option while the user types. See
/// [Commands](crate::guide::commands#autocomplete).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::autocomplete;

/// An enum of fixed Choices, for Options, selects and radio groups. See
/// [Commands](crate::guide::commands#choices).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::Choice;

/// A form shown as a modal. See [Components and modals](crate::guide::components#forms).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::Modal;

/// Markup for message and modal layouts. Every tag and attribute is listed in
/// [Views](crate::guide::views#tag-reference).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::rsx;

/// A layout rendered from Props, with inline handlers. See [Views](crate::guide::views).
#[cfg(feature = "macros")]
#[doc(inline)]
pub use composure_macros::view;

/// Something Discord renders as a mention in message content: `<@id>` for users, `<#id>` for
/// channels, `<@&id>` for roles.
pub trait Mentionable {
    /// The mention markup, e.g. `<@80351110224678912>`
    fn to_mention(&self) -> String;
}
