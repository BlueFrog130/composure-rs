//! Example bot: every command defined once with `#[command]`, collected into a Command Set. The
//! same commands are served on Cloudflare Workers (`examples/cloudflare`) and on a hosted server
//! with tokio (`examples/tokio`), each adding its platform's own through [`example_bot!`] and
//! syncing its set with its `sync` bin.
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
use composure::command::Theme;
pub use events::*;
pub use interactive::*;
pub use views::*;

/// Colors and branding filled into every embed the bot answers with, errors included
pub const THEME: Theme = Theme::new()
    .with_primary(0xf5c2e7)
    .with_footer("composure example", None);

/// Declares a Command Set serving every shared Command, component, modal and View, plus `extra`
/// Commands: each example adds what only its platform can do and syncs that set
/// (`examples/cloudflare`: Workers KV, `examples/tokio`: local files, process data, blocking work).
///
/// ```ignore
/// composure_example_bot::example_bot! {
///     pub struct CloudflareBot = [Note];
/// }
/// ```
#[macro_export]
macro_rules! example_bot {
    ($(#[$attr:meta])* $vis:vis struct $name:ident = [$($extra:path),* $(,)?];) => {
        ::composure::commands! {
            $(#[$attr])*
            $vis struct $name = [
                $crate::hello, $crate::echo, $crate::ice_cream, $crate::fruit, $crate::greet,
                $crate::Mod, $crate::avatar, $crate::word_count, $crate::whoami, $crate::remind,
                $crate::countdown, $crate::announce, $crate::thumbs_up, $crate::dm_me,
                $crate::report, $crate::command_info, $crate::export, $crate::counter,
                $crate::roll, $crate::colors, $crate::feedback, $crate::card, $crate::dice,
                $($extra,)*
            ];
            components = [$crate::counter_click, $crate::reroll, $crate::pick_colors];
            modals = [$crate::on_feedback];
            views = [$crate::Card, $crate::Review, $crate::Dice];
            theme = $crate::THEME;
            events = $crate::Events;
        }
    };
}

example_bot! {
    /// The shared commands only, as both examples serve them besides their own
    pub struct Bot = [];
}

/// Workers vars, from wrangler.jsonc
#[cfg(feature = "cloudflare")]
impl Vars for composure_adapter_cloudflare::Worker {
    fn var(&self, name: &str) -> Option<String> {
        self.env().var(name).ok().map(|var| var.to_string())
    }
}

/// The server's environment variables
#[cfg(feature = "tokio")]
impl<T> Vars for composure_adapter_tokio::Host<T> {
    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }
}
