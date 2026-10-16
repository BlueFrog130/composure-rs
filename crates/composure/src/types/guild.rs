use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::{Emoji, Role, Snowflake, Sticker};

/// The commonly sent fields of a [Guild](https://discord.com/developers/docs/resources/guild#guild-object)
#[derive(Debug, Clone, Deserialize)]
pub struct PartialGuild {
    pub id: Snowflake,

    pub name: String,

    /// [Icon hash](https://discord.com/developers/docs/reference#image-formatting)
    pub icon: Option<String>,

    pub owner_id: Option<Snowflake>,

    /// Enabled [guild features](https://discord.com/developers/docs/resources/guild#guild-object-guild-features)
    #[serde(default)]
    pub features: Vec<String>,
}

/// [Guild](https://discord.com/developers/docs/resources/guild#guild-object)
#[derive(Debug, Clone, Deserialize)]
pub struct Guild {
    pub id: Snowflake,

    pub name: String,

    /// [Icon hash](https://discord.com/developers/docs/reference#image-formatting)
    pub icon: Option<String>,

    pub splash: Option<String>,

    pub discovery_splash: Option<String>,

    pub banner: Option<String>,

    pub description: Option<String>,

    pub owner_id: Snowflake,

    pub afk_channel_id: Option<Snowflake>,

    /// AFK timeout in seconds
    pub afk_timeout: u32,

    pub widget_enabled: Option<bool>,

    pub widget_channel_id: Option<Snowflake>,

    pub verification_level: VerificationLevel,

    pub default_message_notifications: DefaultMessageNotificationLevel,

    pub explicit_content_filter: ExplicitContentFilterLevel,

    pub roles: Vec<Role>,

    pub emojis: Vec<Emoji>,

    #[serde(default)]
    pub stickers: Vec<Sticker>,

    /// Enabled [guild features](https://discord.com/developers/docs/resources/guild#guild-object-guild-features)
    pub features: Vec<String>,

    pub mfa_level: MfaLevel,

    /// The app that created the guild, if bot-created
    pub application_id: Option<Snowflake>,

    pub system_channel_id: Option<Snowflake>,

    /// [System channel flags](https://discord.com/developers/docs/resources/guild#guild-object-system-channel-flags)
    pub system_channel_flags: u32,

    pub rules_channel_id: Option<Snowflake>,

    pub max_presences: Option<u32>,

    pub max_members: Option<u32>,

    pub vanity_url_code: Option<String>,

    pub premium_tier: PremiumTier,

    pub premium_subscription_count: Option<u32>,

    pub preferred_locale: String,

    pub public_updates_channel_id: Option<Snowflake>,

    pub max_video_channel_users: Option<u32>,

    pub max_stage_video_channel_users: Option<u32>,

    /// With `with_counts`
    pub approximate_member_count: Option<u32>,

    /// With `with_counts`
    pub approximate_presence_count: Option<u32>,

    pub nsfw_level: NsfwLevel,

    pub premium_progress_bar_enabled: bool,

    pub safety_alerts_channel_id: Option<Snowflake>,
}

/// [Guild Preview](https://discord.com/developers/docs/resources/guild#guild-preview-object)
#[derive(Debug, Clone, Deserialize)]
pub struct GuildPreview {
    pub id: Snowflake,
    pub name: String,
    pub icon: Option<String>,
    pub splash: Option<String>,
    pub discovery_splash: Option<String>,
    pub emojis: Vec<Emoji>,
    pub features: Vec<String>,
    pub approximate_member_count: u32,
    pub approximate_presence_count: u32,
    pub description: Option<String>,
    #[serde(default)]
    pub stickers: Vec<Sticker>,
}

/// [Verification Level](https://discord.com/developers/docs/resources/guild#guild-object-verification-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum VerificationLevel {
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    VeryHigh = 4,
}

/// [Default Message Notification Level](https://discord.com/developers/docs/resources/guild#guild-object-default-message-notification-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum DefaultMessageNotificationLevel {
    AllMessages = 0,
    OnlyMentions = 1,
}

/// [Explicit Content Filter Level](https://discord.com/developers/docs/resources/guild#guild-object-explicit-content-filter-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum ExplicitContentFilterLevel {
    Disabled = 0,
    MembersWithoutRoles = 1,
    AllMembers = 2,
}

/// [MFA Level](https://discord.com/developers/docs/resources/guild#guild-object-mfa-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum MfaLevel {
    None = 0,
    Elevated = 1,
}

/// [Premium Tier](https://discord.com/developers/docs/resources/guild#guild-object-premium-tier)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum PremiumTier {
    None = 0,
    Tier1 = 1,
    Tier2 = 2,
    Tier3 = 3,
}

/// [Guild NSFW Level](https://discord.com/developers/docs/resources/guild#guild-object-guild-nsfw-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum NsfwLevel {
    Default = 0,
    Explicit = 1,
    Safe = 2,
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
