use std::{ops::Deref, sync::Arc};

use composure::{
    Credentials, CredentialsError,
    command::{BackgroundWork, HandlerError, HasData, Platform},
    credentials::BOT_TOKEN_VAR,
    rest::BotClient,
};
use worker::{Context, Env, send::SendWrapper};

use crate::FetchTransport;

/// The [`Credentials`] from the Worker's vars and secrets by their conventional names:
/// `DISCORD_PUBLIC_KEY` (required), `DISCORD_TOKEN` and `DISCORD_APPLICATION_ID` (optional). What
/// [`CloudflareInteractionBot`](crate::CloudflareInteractionBot) reads unless given
/// [`with_credentials`](crate::CloudflareInteractionBot::with_credentials).
pub fn credentials(env: &Env) -> Result<Credentials, CredentialsError> {
    // `var` reads secrets too
    Credentials::from_lookup(|name| env.var(name).ok().map(|var| var.to_string()))
}

/// A Bot Client outside Interactions (cron triggers, queues, ...) from [`credentials`]: the
/// `DISCORD_TOKEN` secret and the `DISCORD_APPLICATION_ID` var (or secret), over
/// [`FetchTransport`]. With Credentials loaded another way, use
/// [`Credentials::bot_client`]`(FetchTransport)`.
///
/// ```ignore
/// #[event(scheduled)]
/// async fn scheduled(_event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
///     let bot = composure_adapter_cloudflare::bot_client(&env).unwrap();
///     bot.send_message(channel_id, "Daily reminder").await.unwrap();
/// }
/// ```
pub fn bot_client(env: &Env) -> worker::Result<BotClient<FetchTransport>> {
    credentials(env)
        .map_err(|e| worker::Error::RustError(e.to_string()))?
        .bot_client(FetchTransport)
        .ok_or_else(|| {
            worker::Error::RustError(
                "a Bot Client needs DISCORD_TOKEN and DISCORD_APPLICATION_ID".into(),
            )
        })
}

/// The State Commands get on Workers: the worker's [`Env`] (also via `Deref`), its [`Context`],
/// which runs Background Work with `wait_until`, and Bot Data `T` (from
/// [`with_data`](crate::CloudflareInteractionBot::with_data); [`HasData`]). Implements
/// [`Platform`], so `ctx.client()`, `ctx.bot()` (with a bot token) and `ctx.defer(..)` work.
///
/// `Send + Sync` like every Platform (ADR 0012): the JS handles are in a [`SendWrapper`], which
/// is sound because a Workers isolate runs on one thread.
pub struct Worker<T = ()> {
    env: SendWrapper<Env>,
    ctx: Arc<SendWrapper<Context>>,
    bot_token: Option<Arc<str>>,
    data: Arc<T>,
}

impl<T> Clone for Worker<T> {
    fn clone(&self) -> Self {
        Self {
            env: self.env.clone(),
            ctx: self.ctx.clone(),
            bot_token: self.bot_token.clone(),
            data: self.data.clone(),
        }
    }
}

impl Worker {
    /// A State outside [`CloudflareInteractionBot`](crate::CloudflareInteractionBot), from an
    /// event's `env` and `ctx`, with the `DISCORD_TOKEN` secret and no Bot Data
    pub fn new(env: Env, ctx: Context) -> Self {
        let bot_token = env
            .var(BOT_TOKEN_VAR)
            .ok()
            .map(|var| var.to_string().into());
        Self::from_parts(env, ctx, bot_token, Arc::new(()))
    }
}

impl<T> Worker<T> {
    pub(crate) fn from_parts(
        env: Env,
        ctx: Context,
        bot_token: Option<Arc<str>>,
        data: Arc<T>,
    ) -> Self {
        Self {
            env: SendWrapper::new(env),
            ctx: Arc::new(SendWrapper::new(ctx)),
            bot_token,
            data,
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

    /// The Bot Data, as built by [`with_data`](crate::CloudflareInteractionBot::with_data)
    pub fn data(&self) -> &T {
        &self.data
    }
}

impl<T> Deref for Worker<T> {
    type Target = Env;

    fn deref(&self) -> &Env {
        &self.env
    }
}

impl<T: Send + Sync + 'static> Platform for Worker<T> {
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

    fn bot_token(&self) -> Option<&str> {
        self.bot_token.as_deref()
    }
}

/// Bot Data: `T`, as built by [`with_data`](crate::CloudflareInteractionBot::with_data)
impl<T> HasData for Worker<T> {
    type Data = T;

    fn data(&self) -> &T {
        &self.data
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
