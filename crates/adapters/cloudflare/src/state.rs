use std::{ops::Deref, rc::Rc};

use composure::command::{BackgroundWork, ErrorKind, HandlerError, Platform};
use worker::{Context, Env, console_debug, console_error};

use crate::FetchTransport;

const BOT_TOKEN_SECRET: &str = "DISCORD_TOKEN";

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

pub(crate) fn log_handler_error(error: &HandlerError) {
    // User errors are expected outcomes the user already sees
    if error.kind() == ErrorKind::User {
        console_debug!("User error: {}", error);
    } else {
        console_error!("Handler error: {}", error);
    }
}
