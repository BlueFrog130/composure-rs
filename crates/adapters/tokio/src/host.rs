use std::{future::Future, ops::Deref, sync::Arc};

use composure::{
    Credentials,
    command::{BackgroundWork, HasData, Platform},
    rest::BotClient,
};
use tokio_util::task::TaskTracker;

use crate::HttpTransport;

/// The State Commands get on a hosted server: your data `T` (also via `Deref`), plus what
/// [`Platform`] needs: the bot token, an [`HttpTransport`], and tokio tasks for Background Work,
/// which graceful shutdown waits for. So `ctx.client()`, `ctx.bot()` (with a bot token) and
/// `ctx.defer(..)` work.
///
/// Cloning is cheap (two `Arc`s); `T` is shared, not cloned. Use [`Host::bot`] for REST outside
/// Interactions.
pub struct Host<T = ()> {
    core: Arc<Core>,
    data: Arc<T>,
}

#[derive(Clone)]
pub(crate) struct Core {
    pub(crate) credentials: Credentials,
    pub(crate) transport: HttpTransport,
    pub(crate) tracker: TaskTracker,
}

impl<T> Clone for Host<T> {
    fn clone(&self) -> Self {
        Self {
            core: self.core.clone(),
            data: self.data.clone(),
        }
    }
}

impl Host {
    pub(crate) fn new(credentials: Credentials) -> Self {
        Self {
            core: Arc::new(Core {
                credentials,
                transport: HttpTransport::new(),
                tracker: TaskTracker::new(),
            }),
            data: Arc::new(()),
        }
    }
}

impl<T> Host<T> {
    /// This Host with `data` instead
    pub(crate) fn with_data<U>(self, data: U) -> Host<U> {
        Host {
            core: self.core,
            data: Arc::new(data),
        }
    }

    pub(crate) fn core_mut(&mut self) -> &mut Core {
        Arc::make_mut(&mut self.core)
    }

    pub(crate) fn tracker(&self) -> &TaskTracker {
        &self.core.tracker
    }

    /// Your data (Bot Data), as given to `with_data`
    pub fn data(&self) -> &T {
        &self.data
    }

    /// The Transport REST calls go through
    pub fn transport(&self) -> HttpTransport {
        self.core.transport.clone()
    }

    /// A Bot Client outside Interactions (startup tasks, timers, ...), or `None` without both a
    /// bot token and an application id (`DISCORD_TOKEN`, `DISCORD_APPLICATION_ID`)
    ///
    /// ```ignore
    /// let host = bot.host();
    /// tokio::spawn(async move {
    ///     let bot = host.bot().expect("DISCORD_TOKEN and DISCORD_APPLICATION_ID are set");
    ///     bot.send_message(channel_id, "Back online").await.ok();
    /// });
    /// ```
    pub fn bot(&self) -> Option<BotClient<HttpTransport>> {
        self.core.credentials.bot_client(self.transport())
    }

    /// Runs `work` as a task graceful shutdown waits for (up to the grace period), like
    /// Background Work
    pub fn spawn(&self, work: impl Future<Output = ()> + Send + 'static) {
        self.core.tracker.spawn(work);
    }
}

impl<T> Deref for Host<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T: Send + Sync + 'static> Platform for Host<T> {
    type Transport = HttpTransport;

    fn transport(&self) -> HttpTransport {
        self.core.transport.clone()
    }

    fn spawn(&self, work: BackgroundWork) {
        // Composure has logged an `Err` inside the work's span already; a panic only ends the
        // inner task, and is logged here
        let task = tokio::spawn(work);
        self.core.tracker.spawn(async move {
            if let Err(error) = task.await
                && error.is_panic()
            {
                tracing::error!(%error, "background work panicked");
            }
        });
    }

    fn bot_token(&self) -> Option<String> {
        self.core.credentials.bot_token().map(str::to_owned)
    }
}

/// Bot Data: `T`, as given to `with_data`
impl<T> HasData for Host<T> {
    type Data = T;

    fn data(&self) -> &T {
        &self.data
    }
}
