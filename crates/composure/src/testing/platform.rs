use std::sync::{Arc, Mutex};

use crate::command::{BackgroundWork, HandlerError, HasData, Platform};

use super::{MockTransport, block_on};

/// A [`Platform`] for tests: REST goes to a [`MockTransport`], Background Work waits until
/// [`run_background`](Self::run_background), and Bot Data (`T`) is whatever the test gives.
/// Clones share everything, like an Adapter's State.
///
/// For a bot's own traits (a key-value store, say), implement them on `TestPlatform<YourData>` in
/// the bot's crate, or, from an integration test (where the orphan rule forbids that), on a
/// newtype that forwards [`Platform`] and [`HasData`] to a `TestPlatform`.
pub struct TestPlatform<T = ()> {
    transport: Arc<MockTransport>,
    data: Arc<T>,
    bot_token: Option<Arc<str>>,
    work: Arc<Mutex<Vec<BackgroundWork>>>,
}

impl<T> Clone for TestPlatform<T> {
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            data: self.data.clone(),
            bot_token: self.bot_token.clone(),
            work: self.work.clone(),
        }
    }
}

impl Default for TestPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl TestPlatform {
    /// No Bot Data, the bot token `bot-token`, and a [`MockTransport`] answering 200 `{}`
    pub fn new() -> Self {
        Self {
            transport: Arc::default(),
            data: Arc::new(()),
            bot_token: Some(Arc::from("bot-token")),
            work: Arc::default(),
        }
    }
}

impl<T> TestPlatform<T> {
    /// With `data` as Bot Data ([`HasData`])
    pub fn with_data<U>(self, data: U) -> TestPlatform<U> {
        TestPlatform {
            transport: self.transport,
            data: Arc::new(data),
            bot_token: self.bot_token,
            work: self.work,
        }
    }

    /// With `token` as the bot token, or none (`ctx.bot()` then fails)
    pub fn with_bot_token(self, token: Option<&str>) -> Self {
        Self {
            bot_token: token.map(Arc::from),
            ..self
        }
    }

    /// With `transport` answering REST requests
    pub fn with_transport(self, transport: MockTransport) -> Self {
        Self {
            transport: Arc::new(transport),
            ..self
        }
    }

    /// The Transport, to read the requests sent
    pub fn transport(&self) -> &MockTransport {
        &self.transport
    }

    /// How much Background Work is waiting to run
    pub fn queued_work(&self) -> usize {
        self.lock_work().len()
    }

    /// Runs the waiting Background Work in order, as `wait_until` or a spawned task would after the
    /// response is sent, then any it spawns in turn, until none is left; every result
    pub fn run_all(&self) -> Vec<Result<(), HandlerError>> {
        let mut results = Vec::new();
        loop {
            let work: Vec<_> = self.lock_work().drain(..).collect();
            if work.is_empty() {
                return results;
            }
            results.extend(work.into_iter().map(block_on));
        }
    }

    /// [`run_all`](Self::run_all), failing with the first error
    pub fn run_background(&self) -> Result<(), HandlerError> {
        self.run_all().into_iter().collect()
    }

    fn lock_work(&self) -> std::sync::MutexGuard<'_, Vec<BackgroundWork>> {
        self.work
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl<T: Send + Sync + 'static> Platform for TestPlatform<T> {
    type Transport = Arc<MockTransport>;

    fn transport(&self) -> Arc<MockTransport> {
        self.transport.clone()
    }

    fn spawn(&self, work: BackgroundWork) {
        self.lock_work().push(work);
    }

    fn bot_token(&self) -> Option<&str> {
        self.bot_token.as_deref()
    }
}

impl<T: Send + Sync> HasData for TestPlatform<T> {
    type Data = T;

    fn data(&self) -> &T {
        &self.data
    }
}
