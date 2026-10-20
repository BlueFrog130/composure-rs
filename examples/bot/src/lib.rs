//! Example bot: every command defined once with `#[command]`, collected into an App ([`Bot`]).
//! The same commands are served on Cloudflare Workers (`examples/cloudflare`) and on a hosted
//! server with tokio (`examples/tokio`), each listing [`Bot`] in its own App next to its
//! platform's commands and syncing that with its `sync` bin.
//!
//! - [`basics`]: answering with text, embeds, Options, Choices, State, Groups, context menus,
//!   themed embeds
//! - [`api`]: REST calls with the bot token, Deferred Responses, follow-ups, file uploads
//! - [`interactive`]: buttons, select menus, and modals with `#[component]` / `#[modal]`
//! - [`views`]: Components V2 layouts and modals as `rsx!` markup with inline handlers (`#[view]`)
//! - [`events`]: Webhook Events (app installs)

pub mod api;
pub mod basics;
pub mod events;
pub mod interactive;
pub mod views;

pub use api::*;
pub use basics::*;
use composure::{app, command::Theme};
pub use events::*;
pub use interactive::*;
pub use views::*;

/// Colors and branding filled into every embed the bot answers with, errors included
pub const THEME: Theme = Theme::new()
    .with_primary(0xf5c2e7)
    .with_footer("composure example", None);

/// Every shared Command, component, modal and View, one App per module, with the theme and the
/// Webhook Events. Each example lists it in its own App next to what only its platform can do
/// (`examples/cloudflare`: Workers KV, `examples/tokio`: local files, process data, blocking work)
/// and syncs that.
#[app(
    handlers(Basics, Api, Interactive, Views),
    theme = THEME,
    events = Events
)]
pub struct Bot;
