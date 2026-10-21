//! Cloudflare Workers: the Worker's entry points (`fetch` for Interactions and Webhook Events,
//! `scheduled` for release announcements) and Workers KV as the [`Store`].
//!
//! KV calls are JS promises, not `Send`, while the [`Store`] futures must be (ADR 0012): they are
//! wrapped in `SendFuture`, which is sound since a Worker runs on one thread.

use std::time::Duration;

use composure::command::HandlerError;
use composure_adapter_cloudflare::{CloudflareInteractionBot, FetchTransport, Worker, bot_client};
use tracing::{Instrument, level_filters::LevelFilter};
use worker::{
    Context, Env, Request, Response, ScheduleContext, ScheduledEvent, event,
    kv::KvStore,
    send::{SendFuture, SendWrapper},
};

use crate::{ComposureBot, Config, Store, notify};

/// The KV binding in wrangler.toml
pub const KV_BINDING: &str = "COMPOSURE";

/// Workers KV's shortest expiration
const MIN_TTL: u64 = 60;

/// The bot's KV namespace
pub struct Kv(SendWrapper<KvStore>);

impl Kv {
    /// The [`KV_BINDING`] namespace
    pub fn from_env(env: &Env) -> Result<Kv, HandlerError> {
        env.kv(KV_BINDING)
            .map(|kv| Kv(SendWrapper::new(kv)))
            .map_err(HandlerError::new)
    }
}

impl Store for Kv {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, HandlerError> {
        SendFuture::new(self.0.get(key).bytes())
            .await
            .map_err(HandlerError::new)
    }

    async fn put(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<Duration>,
    ) -> Result<(), HandlerError> {
        SendFuture::new(async move {
            let mut put = self.0.put_bytes(key, &value)?;
            if let Some(ttl) = ttl {
                put = put.expiration_ttl(ttl.as_secs().max(MIN_TTL));
            }
            put.execute().await
        })
        .await
        .map_err(HandlerError::new)
    }
}

/// Handlers reach KV through their State
impl Store for Worker<Config> {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, HandlerError> {
        Kv::from_env(self.env())?.get(key).await
    }

    async fn put(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<Duration>,
    ) -> Result<(), HandlerError> {
        Kv::from_env(self.env())?.put(key, value, ttl).await
    }
}

/// The Bot Data, from vars and secrets
fn config(env: &Env) -> Result<Config, String> {
    // `var` reads secrets too
    Config::from_lookup(|name| env.var(name).ok().map(|var| var.to_string()))
}

/// Runs once per isolate: logs go to Workers Logs with each Interaction's span fields
#[event(start)]
fn start() {
    composure_adapter_cloudflare::tracing::init_with_level(LevelFilter::INFO);
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> worker::Result<Response> {
    CloudflareInteractionBot::new(req, env, ctx)
        .with_handler(ComposureBot)
        .with_data(async |env: Env| config(&env).map_err(Into::into))
        .process()
        .await
}

/// Cron trigger (`triggers.crons` in wrangler.toml): announces new releases
#[event(scheduled)]
async fn scheduled(event: ScheduledEvent, env: Env, _ctx: ScheduleContext) {
    // `instrument`, not `entered()`: a guard held across `.await` would leak the span onto other
    // requests the isolate runs meanwhile
    let span = tracing::info_span!("announce", cron = event.cron());
    async {
        let result = async {
            let config = config(&env).map_err(HandlerError::new)?;
            let bot = bot_client(&env).map_err(HandlerError::new)?;
            let store = Kv::from_env(&env)?;
            notify::announce_new(&bot, &FetchTransport, &store, &config).await
        }
        .await;
        match result {
            Ok(versions) if versions.is_empty() => tracing::debug!("nothing new to announce"),
            Ok(versions) => tracing::info!(?versions, "announced"),
            Err(error) => tracing::error!(%error, "announcing failed"),
        }
    }
    .instrument(span)
    .await
}
