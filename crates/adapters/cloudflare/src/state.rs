use std::{ops::Deref, rc::Rc};

use composure::{
    command::{BackgroundWork, ErrorKind, HandlerError, Platform},
    rest::BotClient,
    types::Snowflake,
};
use worker::{Context, Env};

use crate::FetchTransport;

const BOT_TOKEN_SECRET: &str = "DISCORD_TOKEN";
const APPLICATION_ID: &str = "DISCORD_APPLICATION_ID";

/// A Bot Client outside Interactions (cron triggers, queues, ...): the `DISCORD_TOKEN` secret and
/// the `DISCORD_APPLICATION_ID` var (or secret), over [`FetchTransport`].
///
/// ```ignore
/// #[event(scheduled)]
/// async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
///     let bot = composure_adapter_cloudflare::bot_client(&env).unwrap();
///     bot.send_message(channel_id, "Daily reminder").await.unwrap();
/// }
/// ```
pub fn bot_client(env: &Env) -> worker::Result<BotClient<FetchTransport>> {
    let token = env.secret(BOT_TOKEN_SECRET)?.to_string();
    let application_id = env
        .var(APPLICATION_ID)
        .map(|var| var.to_string())
        .or_else(|_| env.secret(APPLICATION_ID).map(|secret| secret.to_string()))?;
    let application_id: Snowflake = application_id
        .parse()
        .map_err(|_| worker::Error::RustError(format!("{APPLICATION_ID} is not an id")))?;
    Ok(BotClient::new(FetchTransport, &token, application_id))
}

/// The State Commands get on Workers: the worker's [`Env`] (also via `Deref`) plus its
/// [`Context`], which runs Background Work with `wait_until`. Implements [`Platform`], so
/// `ctx.client()`, `ctx.bot()` (with a `DISCORD_TOKEN` secret) and `ctx.defer(..)` work.
#[derive(Clone)]
pub struct Worker {
    env: Env,
    ctx: Rc<Context>,
}

impl Worker {
    pub fn new(env: Env, ctx: Context) -> Self {
        Self {
            env,
            ctx: Rc::new(ctx),
        }
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    pub fn context(&self) -> &Context {
        &self.ctx
    }
}

impl Deref for Worker {
    type Target = Env;

    fn deref(&self) -> &Env {
        &self.env
    }
}

impl Platform for Worker {
    type Transport = FetchTransport;

    fn transport(&self) -> FetchTransport {
        FetchTransport
    }

    fn spawn(&self, work: BackgroundWork) {
        let task = async move {
            if let Err(error) = work.await {
                log_handler_error(&error);
            }
        };
        // Composure spawns inside the work's span; keep it for the error log too
        #[cfg(feature = "tracing")]
        let task = ::tracing::Instrument::in_current_span(task);
        self.ctx.wait_until(task);
    }

    fn bot_token(&self) -> Option<String> {
        self.env
            .secret(BOT_TOKEN_SECRET)
            .ok()
            .map(|secret| secret.to_string())
    }
}

pub(crate) fn log_handler_error(error: &HandlerError) {
    match error.kind() {
        // Expected outcomes the user already sees
        ErrorKind::User => debug!("user error: {error}"),
        // Usually Commands out of sync with Discord
        ErrorKind::UnknownCommand | ErrorKind::InvalidOption => warn!("{error}"),
        ErrorKind::Handler => error!("handler failed: {error}"),
    }
}
