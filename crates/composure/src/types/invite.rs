use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::{PartialChannel, PartialGuild, Snowflake, User};

/// [Invite](https://discord.com/developers/docs/resources/invite#invite-object), with
/// [metadata](https://discord.com/developers/docs/resources/invite#invite-metadata-object) when
/// listed from a channel or guild
#[derive(Debug, Clone, Deserialize)]
pub struct Invite {
    /// 0 guild, 1 group DM, 2 friend
    #[serde(rename = "type", default)]
    pub kind: u8,

    /// The `discord.gg/<code>` code
    pub code: String,

    /// The guild it invites to
    pub guild: Option<PartialGuild>,

    /// The channel it invites to
    pub channel: Option<PartialChannel>,

    /// Who created it
    pub inviter: Option<User>,

    /// What a voice channel invite targets
    pub target_type: Option<InviteTargetType>,

    /// Whose stream it shows, for stream invites
    pub target_user: Option<User>,

    /// With `with_counts`
    pub approximate_member_count: Option<u32>,

    /// With `with_counts`
    pub approximate_presence_count: Option<u32>,

    /// ISO 8601; `None` for invites that never expire
    pub expires_at: Option<String>,

    /// Metadata: times used
    pub uses: Option<u32>,

    /// Metadata: use limit, 0 for unlimited
    pub max_uses: Option<u32>,

    /// Metadata: lifetime in seconds, 0 for forever
    pub max_age: Option<u32>,

    /// Metadata: membership is temporary
    pub temporary: Option<bool>,

    /// Metadata: ISO 8601
    pub created_at: Option<String>,
}

/// [Invite Target Type](https://discord.com/developers/docs/resources/invite#invite-object-invite-target-types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum InviteTargetType {
    /// A user's stream in a voice channel
    Stream = 1,
    /// An embedded application (an Activity) in a voice channel
    EmbeddedApplication = 2,
}

/// [Ban](https://discord.com/developers/docs/resources/guild#ban-object)
#[derive(Debug, Clone, Deserialize)]
pub struct Ban {
    /// Why they were banned
    pub reason: Option<String>,
    /// The banned user
    pub user: User,
}

/// Result of a [bulk ban](https://discord.com/developers/docs/resources/guild#bulk-guild-ban)
#[derive(Debug, Clone, Deserialize)]
pub struct BulkBan {
    /// Users who were banned
    pub banned_users: Vec<Snowflake>,
    /// Users who could not be banned (missing permissions, already banned, ...)
    pub failed_users: Vec<Snowflake>,
}
