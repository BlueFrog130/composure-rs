//! Async Discord REST client over an Adapter-provided [`Transport`] (ADR 0004, 0005).
//!
//! [`InteractionClient`] needs only an Interaction's token; [`BotClient`] needs the bot token and
//! gains Endpoints per `rest-*` feature.

mod bot;
mod client;
#[cfg(feature = "rest-commands")]
mod commands;
mod error;
mod interaction;
mod multipart;
#[cfg(feature = "reqwest")]
mod reqwest;
mod response;
mod route;
#[cfg(test)]
mod spec;
#[cfg(test)]
pub(crate) mod testing;
mod transport;

pub use bot::BotClient;
pub(crate) use client::{Body, Rest};
pub use client::{DEFAULT_MAX_RATE_LIMIT_WAIT, DISCORD_API};
#[cfg(feature = "rest-commands")]
pub use commands::CommandScope;
pub use error::{ApiError, Error, JsonErrorCode, RateLimited, Result};
pub use interaction::{CreateFollowup, EditMessage, InteractionClient};
#[cfg(feature = "reqwest")]
pub use reqwest::ReqwestTransport;
pub use response::Response;
pub use route::Route;
pub use transport::{Method, RawResponse, Request, Transport, TransportError};
