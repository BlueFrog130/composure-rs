//! Async Discord REST client over an Adapter-provided [`Transport`] (ADR 0004, 0005).
//!
//! [`InteractionClient`] needs only an Interaction's token; [`BotClient`] needs the bot token and
//! gains Endpoints per `rest-*` feature.

#[cfg(feature = "rest-application")]
mod application;
#[cfg(feature = "rest-audit-log")]
mod audit_log;
#[cfg(feature = "rest-automod")]
mod automod;
#[cfg(feature = "rest-bans")]
mod bans;
mod bot;
#[cfg(feature = "rest-channels")]
mod channels;
mod client;
#[cfg(feature = "rest-commands")]
mod commands;
#[cfg(feature = "rest-emojis")]
mod emojis;
mod error;
#[cfg(feature = "rest-guilds")]
mod guilds;
mod interaction;
#[cfg(feature = "rest-invites")]
mod invites;
#[cfg(feature = "rest-members")]
mod members;
#[cfg(feature = "rest-messages")]
mod messages;
#[cfg(feature = "rest-monetization")]
mod monetization;
mod multipart;
#[cfg(feature = "rest-polls")]
mod polls;
#[cfg(feature = "reqwest")]
mod reqwest;
mod response;
#[cfg(feature = "rest-roles")]
mod roles;
mod route;
#[cfg(feature = "rest-scheduled-events")]
mod scheduled_events;
#[cfg(feature = "rest-soundboard")]
mod soundboard;
#[cfg(test)]
mod spec;
#[cfg(feature = "rest-stickers")]
mod stickers;
#[cfg(test)]
pub(crate) mod testing;
#[cfg(feature = "rest-threads")]
mod threads;
mod transport;
#[cfg(feature = "rest-users")]
mod users;
#[cfg(feature = "rest-webhooks")]
mod webhooks;

#[cfg(feature = "rest-application")]
pub use application::EditApplication;
#[cfg(feature = "rest-audit-log")]
pub use audit_log::GetAuditLog;
#[cfg(feature = "rest-automod")]
pub use automod::AutoModerationRuleFields;
#[cfg(feature = "rest-bans")]
pub use bans::GetBans;
pub use bot::BotClient;
#[cfg(feature = "rest-channels")]
pub use channels::{EditChannelPermissions, ModifyChannel};
pub(crate) use client::{Body, Rest};
pub use client::{DEFAULT_MAX_RATE_LIMIT_WAIT, DISCORD_API};
#[cfg(feature = "rest-commands")]
pub use commands::CommandScope;
#[cfg(feature = "rest-emojis")]
pub use emojis::{ApplicationEmojis, CreateEmoji, ModifyEmoji};
pub use error::{ApiError, Error, JsonErrorCode, RateLimited, Result};
#[cfg(feature = "rest-guilds")]
pub use guilds::{ChannelPosition, CreateGuildChannel, GetGuilds, ModifyGuild};
pub use interaction::{CreateFollowup, EditMessage, InteractionClient};
#[cfg(feature = "rest-invites")]
pub use invites::CreateInvite;
#[cfg(feature = "rest-members")]
pub use members::{AddMember, ModifyCurrentMember, ModifyMember};
#[cfg(feature = "rest-messages")]
pub use messages::{CreateMessage, GetMessages};
#[cfg(feature = "rest-monetization")]
pub use monetization::{EntitlementOwner, ListEntitlements};
#[cfg(feature = "reqwest")]
pub use reqwest::ReqwestTransport;
pub use response::Response;
#[cfg(feature = "rest-roles")]
pub use roles::{RoleFields, RolePosition};
pub use route::Route;
#[cfg(feature = "rest-scheduled-events")]
pub use scheduled_events::{GetScheduledEventUsers, ScheduledEventFields};
#[cfg(feature = "rest-soundboard")]
pub use soundboard::{SoundboardSoundFields, SoundboardSounds};
#[cfg(feature = "rest-stickers")]
pub use stickers::{CreateSticker, ModifySticker, StickerPack, StickerPacks};
#[cfg(feature = "rest-threads")]
pub use threads::{ArchivedThreads, StartForumThread, StartThread};
pub use transport::{Method, RawResponse, Request, Transport, TransportError};
#[cfg(feature = "rest-webhooks")]
pub use webhooks::{ExecuteWebhook, WebhookClient, WebhookFields};
