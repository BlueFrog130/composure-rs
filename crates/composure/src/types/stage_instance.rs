//! [Stage instances](https://discord.com/developers/docs/resources/stage-instance): a live stage

use serde::Deserialize;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::Snowflake;

/// [Stage Instance](https://discord.com/developers/docs/resources/stage-instance#stage-instance-object):
/// a stage channel that is live
#[derive(Debug, Clone, Deserialize)]
pub struct StageInstance {
    /// The stage instance's id
    pub id: Snowflake,

    /// The guild
    pub guild_id: Snowflake,

    /// The stage channel
    pub channel_id: Snowflake,

    /// What it is about, 1-120 characters
    pub topic: String,

    /// Who can see it
    pub privacy_level: StagePrivacyLevel,

    /// Whether it is hidden from stage discovery (deprecated)
    #[serde(default)]
    pub discoverable_disabled: bool,

    /// The scheduled event it is for, if any
    pub guild_scheduled_event_id: Option<Snowflake>,
}

/// [Privacy Level](https://discord.com/developers/docs/resources/stage-instance#stage-instance-object-privacy-level)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum StagePrivacyLevel {
    /// Visible publicly (deprecated)
    Public = 1,
    /// Visible to the guild's members
    GuildOnly = 2,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_stage_instances() {
        let stage: StageInstance = serde_json::from_value(json!({
            "id": "840647391636226060", "guild_id": "197038439483310086",
            "channel_id": "733488538393510049", "topic": "Testing Testing, 123", "privacy_level": 2,
            "discoverable_disabled": false, "guild_scheduled_event_id": "947656305244532806",
        }))
        .unwrap();
        assert_eq!(stage.privacy_level, StagePrivacyLevel::GuildOnly);
        assert!(stage.guild_scheduled_event_id.is_some());
    }
}
