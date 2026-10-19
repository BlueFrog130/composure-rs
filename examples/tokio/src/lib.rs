//! The example bot served from a hosted server on tokio: the shared commands (`examples/bot`)
//! plus what only a server has. `src/main.rs` serves it, `src/bin/sync.rs` syncs it.
//!
//! - [`server`]: `/file` (the local filesystem, sandboxed to one directory, with autocomplete),
//!   `/uptime` (process data through `Host<Server>`), `/primes` (a Deferred Response whose
//!   Background Work runs on `spawn_blocking`)

pub mod server;

use composure::app;
use composure_example_bot::{Bot, THEME};
pub use server::Server;
use server::{file, primes, uptime};

/// Every command the bot serves on tokio, synced by `src/bin/sync.rs`
#[app(handlers(Bot, file, uptime, primes), theme = THEME)]
pub struct TokioBot;

/// Where `/file` reads from when `FILES_DIR` is not set: the sample files next to this crate
pub const DEFAULT_FILES_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/files");
