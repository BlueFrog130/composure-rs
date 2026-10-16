//! Example bot: every command defined once with `#[command]`, collected into a Command Set, and
//! served on Cloudflare Workers. `src/bin/sync.rs` syncs the same Command Set with Discord.
//!
//! - [`basics`]: answering with text, embeds, Options, Choices, State, Groups, context menus
//! - [`api`]: REST calls with the bot token, Deferred Responses, follow-ups, file uploads
//! - [`interactive`]: buttons and modals

pub mod api;
pub mod basics;
pub mod interactive;

pub use api::*;
pub use basics::*;
use composure::commands;
use composure_adapter_cloudflare::CloudflareInteractionBot;
pub use interactive::*;
use worker::{Context, Env, Request, Response, event};

commands! {
    /// Every command this bot serves
    pub struct Bot = [
        hello, echo, ice_cream, greet, Mod, avatar, word_count,
        whoami, remind, countdown, announce, thumbs_up, dm_me, report, export,
        counter, feedback,
    ];
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> worker::Result<Response> {
    CloudflareInteractionBot::new(req, env, ctx)
        .with_handler(Bot.with_fallback(Components))
        .process()
        .await
}
