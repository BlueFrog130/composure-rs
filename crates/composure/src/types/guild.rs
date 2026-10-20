use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::{Emoji, Role, Snowflake, Sticker};

/// The commonly sent fields of a [Guild](https://discord.com/developers/docs/resources/guild#guild-object)
#[derive(Debug, Clone, Deserialize)]
pub struct PartialGuild {
    /// The guild's id
    pub id: Snowflake,

    /// 2-100 characters
    pub name: String,

    /// [Icon hash](https://discord.com/developers/docs/reference#image-formatting)
    pub icon: Option<String>,

    /// The owner's user id
    pub owner_id: Option<Snowflake>,

    /// Enabled [guild features](https://discord.com/developers/docs/resources/guild#guild-object-guild-features)
    #[serde(default)]
    pub features: Vec<String>,
}

/// [Guild](https://discord.com/developers/docs/resources/guild#guild-object)
#[derive(Debug, Clone, Deserialize)]
pub struct Guild {
    /// The guild's id
    pub id: Snowflake,

    /// 2-100 characters
    pub name: String,

    /// [Icon hash](https://discord.com/developers/docs/reference#image-formatting)
    pub icon: Option<String>,

    /// [Splash hash](https://discord.com/developers/docs/reference#image-formatting): the invite background
    pub splash: Option<String>,

    /// Discovery splash hash, for discoverable guilds
    pub discovery_splash: Option<String>,

    /// Banner hash
    pub banner: Option<String>,

    /// Description, for discoverable guilds
    pub description: Option<String>,

    /// The owner's user id
    pub owner_id: Snowflake,

    /// The AFK voice channel
    pub afk_channel_id: Option<Snowflake>,

    /// AFK timeout in seconds
    pub afk_timeout: u32,

    /// Whether the server widget is enabled
    pub widget_enabled: Option<bool>,

    /// The channel the widget's invite points to
    pub widget_channel_id: Option<Snowflake>,

    /// What members need before they can talk
    pub verification_level: VerificationLevel,

    /// Default notifications for members
    pub default_message_notifications: DefaultMessageNotificationLevel,

    /// Whose messages are scanned for explicit media
    pub explicit_content_filter: ExplicitContentFilterLevel,

    /// The guild's roles
    pub roles: Vec<Role>,

    /// The guild's custom emojis
    pub emojis: Vec<Emoji>,

    /// The guild's custom stickers
    #[serde(default)]
    pub stickers: Vec<Sticker>,

    /// Enabled [guild features](https://discord.com/developers/docs/resources/guild#guild-object-guild-features)
    pub features: Vec<String>,

    /// Whether moderators need 2FA
    pub mfa_level: MfaLevel,

    /// The app that created the guild, if bot-created
    pub application_id: Option<Snowflake>,

    /// Where system messages (welcome, boosts) are posted
    pub system_channel_id: Option<Snowflake>,

    /// [System channel flags](https://discord.com/developers/docs/resources/guild#guild-object-system-channel-flags)
    pub system_channel_flags: u32,

    /// The rules channel, in community guilds
    pub rules_channel_id: Option<Snowflake>,

    /// Most presences (`None` except for the largest guilds)
    pub max_presences: Option<u32>,

    /// Most members
    pub max_members: Option<u32>,

    /// Vanity invite code
    pub vanity_url_code: Option<String>,

    /// Boost level
    pub premium_tier: PremiumTier,

    /// Number of boosts
    pub premium_subscription_count: Option<u32>,

    /// Locale of a community guild, `en-US` by default
    pub preferred_locale: String,

    /// Where Discord's notices to admins go, in community guilds
    pub public_updates_channel_id: Option<Snowflake>,

    /// Most users in a video channel
    pub max_video_channel_users: Option<u32>,

    /// Most users in a stage video channel
    pub max_stage_video_channel_users: Option<u32>,

    /// With `with_counts`
    pub approximate_member_count: Option<u32>,

    /// With `with_counts`
    pub approximate_presence_count: Option<u32>,

    /// Age restriction level
    pub nsfw_level: NsfwLevel,

    /// Whether the boost progress bar is shown
    pub premium_progress_bar_enabled: bool,

    /// Where Discord's safety alerts go, in community guilds
    pub safety_alerts_channel_id: Option<Snowflake>,
}

/// [Guild Preview](https://discord.com/developers/docs/resources/guild#guild-preview-object)
#[derive(Debug, Clone, Deserialize)]
pub struct GuildPreview {
    /// The guild's id
    pub id: Snowflake,
    /// 2-100 characters
    pub name: String,
    /// Icon hash
    pub icon: Option<String>,
    /// Splash hash
    pub splash: Option<String>,
    /// Discovery splash hash
    pub discovery_splash: Option<String>,
    /// Custom emojis
    pub emojis: Vec<Emoji>,
    /// Enabled guild features
    pub features: Vec<String>,
    /// Approximate member count
    pub approximate_member_count: u32,
    /// Approximate online member count
    pub approximate_presence_count: u32,
    /// Description, for discoverable guilds
    pub description: Option<String>,
    /// Custom stickers
    #[serde(default)]
    pub stickers: Vec<Sticker>,
}

/// [Verification Level](https://discord.com/developers/docs/resources/guild#guild-object-verification-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum VerificationLevel {
    /// No restriction
    None = 0,
    /// A verified email
    Low = 1,
    /// Registered for more than 5 minutes
    Medium = 2,
    /// A member of the server for more than 10 minutes
    High = 3,
    /// A verified phone number
    VeryHigh = 4,
}

/// [Default Message Notification Level](https://discord.com/developers/docs/resources/guild#guild-object-default-message-notification-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum DefaultMessageNotificationLevel {
    /// Notify members of every message
    AllMessages = 0,
    /// Notify members only of mentions
    OnlyMentions = 1,
}

/// [Explicit Content Filter Level](https://discord.com/developers/docs/resources/guild#guild-object-explicit-content-filter-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum ExplicitContentFilterLevel {
    /// Scan no media
    Disabled = 0,
    /// Scan media from members without roles
    MembersWithoutRoles = 1,
    /// Scan media from every member
    AllMembers = 2,
}

/// [MFA Level](https://discord.com/developers/docs/resources/guild#guild-object-mfa-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum MfaLevel {
    /// No 2FA requirement for moderation
    None = 0,
    /// Moderators need 2FA
    Elevated = 1,
}

/// [Premium Tier](https://discord.com/developers/docs/resources/guild#guild-object-premium-tier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum PremiumTier {
    /// No boost level
    None = 0,
    /// Boost level 1
    Tier1 = 1,
    /// Boost level 2
    Tier2 = 2,
    /// Boost level 3
    Tier3 = 3,
}

/// [Guild NSFW Level](https://discord.com/developers/docs/resources/guild#guild-object-guild-nsfw-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum NsfwLevel {
    /// Not rated
    Default = 0,
    /// Explicit
    Explicit = 1,
    /// Safe
    Safe = 2,
    /// Age-restricted
    AgeRestricted = 3,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn deserializes_a_rest_guild() {
        let guild: Guild = serde_json::from_value(json!({
            "id": "1", "name": "Home", "icon": null, "description": null, "home_header": null,
            "splash": null, "discovery_splash": null, "features": ["COMMUNITY"], "banner": null,
            "owner_id": "2", "application_id": null, "region": "deprecated", "afk_channel_id": null,
            "afk_timeout": 300, "system_channel_id": "3", "system_channel_flags": 0,
            "widget_enabled": false, "widget_channel_id": null, "verification_level": 1,
            "roles": [{ "id": "1", "name": "@everyone", "permissions": "1024", "position": 0,
                        "color": 0, "hoist": false, "managed": false, "mentionable": false,
                        "icon": null, "unicode_emoji": null, "flags": 0 }],
            "default_message_notifications": 1, "mfa_level": 0, "explicit_content_filter": 2,
            "max_presences": null, "max_members": 500000, "max_stage_video_channel_users": 50,
            "max_video_channel_users": 25, "vanity_url_code": null, "premium_tier": 0,
            "premium_subscription_count": 0, "preferred_locale": "en-US",
            "rules_channel_id": null, "safety_alerts_channel_id": null,
            "public_updates_channel_id": null, "premium_progress_bar_enabled": false,
            "nsfw": false, "nsfw_level": 0, "emojis": [], "stickers": [],
            "approximate_member_count": 12,
        }))
        .unwrap();
        assert_eq!(guild.verification_level, VerificationLevel::Low);
        assert_eq!(guild.roles[0].name, "@everyone");
        assert_eq!(guild.approximate_member_count, Some(12));
    }
}
