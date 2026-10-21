//! The example bot served on Cloudflare Workers: the shared commands (`examples/bot`) plus what
//! only Workers has, and a cron trigger using the Bot Client outside Interactions.
//!
//! - [`notes`]: `/note save|show|forget`, a per-user note in Workers KV

pub mod notes;

use composure::{app, types::Snowflake};
use composure_adapter_cloudflare::{CloudflareInteractionBot, bot_client};
use composure_example_bot::{Bot, Config};
use notes::Note;
use tracing::{Instrument, level_filters::LevelFilter};
use worker::{Context, Env, Request, Response, ScheduleContext, ScheduledEvent, event};

/// Every command the bot serves on Workers, synced by `src/bin/sync.rs`
#[app(handlers(Bot, Note))]
pub struct CloudflareBot;

/// Runs once per isolate: logs go to the Workers console as objects Workers Logs indexes, with
/// the Interaction's id, type, name, guild and user on every line logged while handling it
#[event(start)]
fn start() {
    composure_adapter_cloudflare::tracing::init_with_level(LevelFilter::DEBUG);
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> worker::Result<Response> {
    CloudflareInteractionBot::new(req, env, ctx)
        .with_handler(CloudflareBot)
        // Bot Data, built on the isolate's first request: the GREETING var from wrangler.toml
        .with_data(async |env: Env| {
            Ok(Config {
                greeting: env.var("GREETING")?.to_string(),
            })
        })
        .process()
        .await
}

/// Cron trigger (`triggers.crons` in wrangler.toml): posts a heartbeat to the channel in the
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
    // `instrument`, not `entered()`: a guard held across `.await` would leak the span onto other
    // requests the isolate runs meanwhile
    let span = tracing::info_span!("heartbeat", cron = event.cron(), %channel);
    async {
        let result = match bot_client(&env) {
            Ok(bot) => bot
                .send_message(channel, &format!("Still running (`{}`)", event.cron()))
                .await
                .map(drop)
                .map_err(|e| e.to_string()),
            Err(e) => Err(e.to_string()),
        };
        match result {
            Ok(()) => tracing::info!("heartbeat sent"),
            Err(error) => tracing::error!(%error, "heartbeat failed"),
        }
    }
    .instrument(span)
    .await
}
