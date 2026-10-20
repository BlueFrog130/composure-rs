use std::{ops::Deref, sync::Arc};

use composure::{
    adapter::{APPLICATION_ID_VAR as APPLICATION_ID, BOT_TOKEN_VAR as BOT_TOKEN_SECRET},
    command::{BackgroundWork, HandlerError, Platform},
    rest::BotClient,
    types::Snowflake,
};
use worker::{Context, Env, send::SendWrapper};

use crate::FetchTransport;

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
///
/// `Send + Sync` like every Platform (ADR 0012): the JS handles are in a [`SendWrapper`], which
/// is sound because a Workers isolate runs on one thread.
#[derive(Clone)]
pub struct Worker {
    env: SendWrapper<Env>,
    ctx: Arc<SendWrapper<Context>>,
}

impl Worker {
    /// The State for one request, from the `fetch` event's `env` and `ctx`
    pub fn new(env: Env, ctx: Context) -> Self {
        Self {
            env: SendWrapper::new(env),
            ctx: Arc::new(SendWrapper::new(ctx)),
        }
    }

    /// The Worker's bindings: vars, secrets, KV, D1, ... (also through `Deref`)
    pub fn env(&self) -> &Env {
        &self.env
    }

    /// The request's execution context, which runs Background Work (`wait_until`)
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
        self.ctx.wait_until(async move {
            if let Err(error) = work.await {
                log_handler_error(&error);
            }
        });
    }

    fn bot_token(&self) -> Option<String> {
        self.env
            .secret(BOT_TOKEN_SECRET)
            .ok()
            .map(|secret| secret.to_string())
    }
}

/// Without `tracing`, logs a handler's error to the console. With it Composure has already logged
/// it inside the Interaction's (or Background Work's, or event's) span, at the level its kind
/// deserves, so this does nothing.
#[cfg(not(feature = "tracing"))]
pub(crate) fn log_handler_error(error: &HandlerError) {
    use composure::command::ErrorKind;

    match error.kind() {
        // Expected outcomes the user already sees
        ErrorKind::User => debug!("user error", error = error),
        // Usually Commands out of sync with Discord
        ErrorKind::UnknownCommand | ErrorKind::InvalidOption => {
            warn!("interaction failed", error = error)
        }
        ErrorKind::Handler => error!("handler failed", error = error),
    }
}

#[cfg(feature = "tracing")]
pub(crate) fn log_handler_error(_error: &HandlerError) {}
