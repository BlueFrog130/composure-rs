//! The Composure guide: how to build a bot, chapter by chapter. These modules hold no code, only
//! documentation; every example in them is compiled as a test.
//!
//! 1. [Getting started](getting_started): crates, features, a first bot, serving and syncing it
//! 2. [Commands](commands): `#[command]`, Options, Choices, Groups, context menus,
//!    autocomplete, Scopes
//! 3. [Responses](responses): what handlers return; ephemeral messages, embeds, files, errors
//! 4. [Components and modals](components): buttons, selects and forms with `#[component]`,
//!    `#[modal]` and `#[derive(Modal)]`, state in the `custom_id`
//! 5. [Views](views): `rsx!` markup and `#[view]`, with a reference of every tag and attribute
//! 6. [Themes](theming): colors and branding on embeds and containers
//! 7. [REST and deferring](deferring): calling Discord's API, follow-ups, Deferred Responses
//! 8. [Webhook Events](events): app installs, entitlements, lobby messages
//! 9. [Platforms](platforms): Adapters, State, platform-specific Commands, logging
//! 10. [Testing](testing): running handlers without Discord
//!
//! Terms like *Command*, *Option*, *App*, *Route*, *Scope* and *Sync* are capitalized when they
//! mean Composure's concept of them; see `CONTEXT.md` in the repository for the glossary, and
//! `docs/adr/` for why things are the way they are.

#[doc = include_str!("guide/getting_started.md")]
pub mod getting_started {}

#[doc = include_str!("guide/commands.md")]
pub mod commands {}

#[doc = include_str!("guide/responses.md")]
pub mod responses {}

#[doc = include_str!("guide/components.md")]
pub mod components {}

#[doc = include_str!("guide/views.md")]
pub mod views {}

#[doc = include_str!("guide/theming.md")]
pub mod theming {}

#[doc = include_str!("guide/deferring.md")]
pub mod deferring {}

#[doc = include_str!("guide/events.md")]
pub mod events {}

#[doc = include_str!("guide/platforms.md")]
pub mod platforms {}

#[doc = include_str!("guide/testing.md")]
pub mod testing {}
