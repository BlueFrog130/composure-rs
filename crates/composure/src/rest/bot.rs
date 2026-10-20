use std::time::Duration;

use crate::types::Snowflake;

use super::{Rest, Transport};

/// REST client authorized by the bot token. Endpoints are added per resource by the `rest-*`
/// features.
#[derive(Debug, Clone)]
pub struct BotClient<T> {
    pub(crate) rest: Rest<T>,
    pub(crate) application_id: Snowflake,
}

impl<T: Transport> BotClient<T> {
    /// `token` is the bot token, without the `Bot ` prefix
    pub fn new(transport: T, token: &str, application_id: Snowflake) -> Self {
        Self {
            rest: Rest::new(transport, Some(format!("Bot {token}"))),
            application_id,
        }
    }

    /// Sends requests to `base` instead of [`DISCORD_API`](super::DISCORD_API) (proxies, tests)
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.rest.set_base(base.into());
        self
    }

    /// Longest single rate-limit wait before giving up with
    /// [`Error::RateLimited`](super::Error::RateLimited). Raise it for deploy-time tools.
    pub fn with_max_rate_limit_wait(mut self, wait: Duration) -> Self {
        self.rest.set_max_rate_limit_wait(wait);
        self
    }

    /// The application the client acts for
    pub fn application_id(&self) -> Snowflake {
        self.application_id
    }

    /// The Transport requests go through
    pub fn transport(&self) -> &T {
        self.rest.transport()
    }
}
