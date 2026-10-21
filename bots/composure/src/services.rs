//! What the bot needs from its platform, beyond a [`Platform`]: Bot Data ([`Config`]) and a
//! key-value [`Store`]. Workers KV on Cloudflare ([`crate::worker`]), a map in tests.

use std::{future::Future, time::Duration};

use composure::command::{HandlerError, HasData, Platform};

use crate::Config;

/// A key-value store of bytes
pub trait Store: Send + Sync {
    /// The value at `key`, if any
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Vec<u8>>, HandlerError>> + Send;

    /// Sets `key`, expiring after `ttl` if given
    fn put(
        &self,
        key: &str,
        value: Vec<u8>,
        ttl: Option<Duration>,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send;
}

/// The State every handler takes: `Ctx<impl Services>`. `Clone + 'static` so Background Work
/// can own it.
pub trait Services: Platform + HasData<Data = Config> + Store + Clone + 'static {}

impl<T: Platform + HasData<Data = Config> + Store + Clone + 'static> Services for T {}
