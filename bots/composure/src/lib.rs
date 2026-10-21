//! Composure's own Discord bot, served on Cloudflare Workers: announces each release, pages
//! through the changelog, and searches the guide. Everything but [`worker`] is
//! platform-agnostic, written against [`Services`], so `tests/` runs it with a stand-in State.
//!
//! - [`changelog`]: `/changelog`, `/compare` and "Explain release", one paginated View
//! - [`notify`]: `/notify setup|remove|status|preview`, release announcements (cron)
//! - [`docs`]: `/docs`, guide and API search with autocomplete
//! - [`crates`]: `/crate`, crates.io stats from Background Work
//! - [`feedback`]: `/feedback`, a modal View posted to the maintainers
//! - [`about`]: `/about`, `/invite`
//! - [`events`]: install logging and the installer's welcome DM
//! - [`ui`]: the tone containers every message is built from

pub mod about;
pub mod changelog;
pub mod config;
pub mod crates;
pub mod docs;
pub mod events;
pub mod feedback;
pub mod http;
pub mod notify;
pub mod releases;
pub mod services;
pub mod ui;
pub mod worker;

use composure::{app, command::Theme};

pub use config::Config;
pub use services::{Services, Store};

/// Build-time facts from `build.rs`: the Composure version, the git commit, the guide index
pub mod build_info {
    include!(concat!(env!("OUT_DIR"), "/build_info.rs"));
}

/// Composure's colors. Messages are [`ui`] containers, which set their accent from these; the
/// only embeds are the framework's error replies, so no author or footer that containers lack.
pub const THEME: Theme = Theme::new().with_primary(0x7c5cff);

/// Every Command, View and handler the bot serves, synced by `src/bin/sync.rs`
#[app(
    handlers(
        changelog::Changelogs,
        notify::Notify,
        docs::docs,
        crates::crate_info,
        feedback::Feedbacks,
        about::about,
        about::invite,
    ),
    theme = THEME,
    events = events::Events
)]
pub struct ComposureBot;
