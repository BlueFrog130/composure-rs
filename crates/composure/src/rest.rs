//! Async Discord REST client over an Adapter-provided [`Transport`] (ADR 0004, 0005).
//!
//! [`InteractionClient`] needs only an Interaction's token; [`BotClient`] needs the bot token and
//! gains Endpoints per `rest-*` feature.

mod bot;
#[cfg(feature = "rest-channels")]
mod channels;
mod client;
#[cfg(feature = "rest-commands")]
mod commands;
mod error;
#[cfg(feature = "rest-guilds")]
mod guilds;
mod interaction;
#[cfg(feature = "rest-members")]
mod members;
#[cfg(feature = "rest-messages")]
mod messages;
mod multipart;
#[cfg(feature = "reqwest")]
mod reqwest;
mod response;
#[cfg(feature = "rest-roles")]
mod roles;
mod route;
#[cfg(test)]
mod spec;
#[cfg(test)]
pub(crate) mod testing;
mod transport;
#[cfg(feature = "rest-users")]
mod users;

pub use bot::BotClient;
#[cfg(feature = "rest-channels")]
pub use channels::{EditChannelPermissions, ModifyChannel};
pub(crate) use client::{Body, Rest};
pub use client::{DEFAULT_MAX_RATE_LIMIT_WAIT, DISCORD_API};
#[cfg(feature = "rest-commands")]
pub use commands::CommandScope;
pub use error::{ApiError, Error, JsonErrorCode, RateLimited, Result};
#[cfg(feature = "rest-guilds")]
pub use guilds::{ChannelPosition, CreateGuildChannel, GetGuilds, ModifyGuild};
pub use interaction::{CreateFollowup, EditMessage, InteractionClient};
#[cfg(feature = "rest-members")]
pub use members::{AddMember, ModifyCurrentMember, ModifyMember};
#[cfg(feature = "rest-messages")]
pub use messages::{CreateMessage, GetMessages};
#[cfg(feature = "reqwest")]
pub use reqwest::ReqwestTransport;
pub use response::Response;
#[cfg(feature = "rest-roles")]
pub use roles::{RoleFields, RolePosition};
pub use route::Route;
pub use transport::{Method, RawResponse, Request, Transport, TransportError};
