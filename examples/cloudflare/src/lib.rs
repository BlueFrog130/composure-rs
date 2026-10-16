//! Example bot: every command defined once with `#[command]`, collected into a Command Set, and
//! served on Cloudflare Workers. `src/bin/sync.rs` syncs the same Command Set with Discord.
//!
//! - [`basics`]: answering with text, embeds, Options, Choices, State, Groups, context menus
//! - [`api`]: REST calls with the bot token, Deferred Responses, follow-ups, file uploads
//! - [`interactive`]: buttons, select menus, and modals with `#[component]` / `#[modal]`
//! - [`events`]: Webhook Events (app installs)

pub mod api;
pub mod basics;
pub mod events;
pub mod interactive;

pub use api::*;
pub use basics::*;
use composure::{commands, types::Snowflake};
use composure_adapter_cloudflare::{CloudflareInteractionBot, bot_client};
pub use events::*;
pub use interactive::*;
use worker::{
    Context, Env, Request, Response, ScheduleContext, ScheduledEvent, console_error, event,
};

commands! {
    /// Every command this bot serves
    pub struct Bot = [
        hello, echo, ice_cream, fruit, greet, Mod, avatar, word_count,
        whoami, remind, countdown, announce, thumbs_up, dm_me, report, command_info, export,
        counter, roll, colors, feedback,
    ];
    components = [counter_click, reroll, pick_colors];
    modals = [on_feedback];
    events = Events;
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> worker::Result<Response> {
    CloudflareInteractionBot::new(req, env, ctx)
        .with_handler(Bot)
        .process()
        .await
}

/// Cron trigger (`triggers.crons` in wrangler.jsonc): posts a heartbeat to the channel in the
/// `STATUS_CHANNEL_ID` var, if set, with the Bot Client from `bot_client`
#[event(scheduled)]
async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    let Some(channel) = env
        .var("STATUS_CHANNEL_ID")
        .ok()
        .and_then(|var| var.to_string().parse::<Snowflake>().ok())
    else {
        return;
    };
    let result = match bot_client(&env) {
        Ok(bot) => bot
            .send_message(channel, &format!("Still running (`{}`)", event.cron()))
            .await
            .map(drop)
            .map_err(|e| e.to_string()),
        Err(e) => Err(e.to_string()),
    };
    if let Err(e) = result {
        console_error!("Heartbeat failed: {}", e);
    }
}
