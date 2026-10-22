//! [Lobbies](https://discord.com/developers/docs/resources/lobby): groups of users an app (often
//! a game, through the Social SDK) creates and talks in; their messages also arrive as Webhook
//! Events

use std::collections::HashMap;

use serde::Deserialize;

use crate::types::{Channel, Snowflake, User};

/// [Lobby Object](https://discord.com/developers/docs/resources/lobby#lobby-object)
#[derive(Debug, Clone, Deserialize)]
pub struct Lobby {
    /// The lobby's id
    pub id: Snowflake,

    /// The application that created it
    pub application_id: Snowflake,

    /// Key-value data the app attached
    pub metadata: Option<HashMap<String, String>>,

    /// The members
    pub members: Vec<LobbyMember>,

    /// The guild channel linked to it, if any
    pub linked_channel: Option<Channel>,

    /// [Lobby flags](https://discord.com/developers/docs/resources/lobby#lobby-object-lobby-flags)
    #[serde(default)]
    pub flags: u32,

    /// Where its Webhook Events go instead of the app's events URL
    pub override_event_webhooks_url: Option<String>,
}

/// [Lobby Member Object](https://discord.com/developers/docs/resources/lobby#lobby-member-object)
#[derive(Debug, Clone, Deserialize)]
pub struct LobbyMember {
    /// The user's id
    pub id: Snowflake,

    /// Key-value data the app attached
    pub metadata: Option<HashMap<String, String>>,

    /// [Lobby member flags](https://discord.com/developers/docs/resources/lobby#lobby-member-object-lobby-member-flags):
    /// 1 = may link the lobby to a channel
    #[serde(default)]
    pub flags: u32,

    /// A name shown alongside theirs in the lobby
    pub additional_name: Option<String>,
}

/// An invite to a lobby's linked guild channel
#[derive(Debug, Clone, Deserialize)]
pub struct LobbyGuildInvite {
    /// The invite code
    pub code: String,
}

/// [Lobby Message](https://discord.com/developers/docs/resources/lobby#lobby-message-object), as
/// sent through REST and the `LOBBY_MESSAGE_*` Webhook Events
#[derive(Debug, Clone, Deserialize)]
pub struct LobbyMessage {
    /// The message's id
    pub id: Snowflake,

    /// The message type
    #[serde(rename = "type")]
    pub t: u8,

    /// The text
    pub content: String,

    /// The lobby
    pub lobby_id: Snowflake,

    /// The lobby's channel
    pub channel_id: Snowflake,

    /// Who sent it
    pub author: User,

    /// Key-value data attached by the game
    pub metadata: Option<HashMap<String, String>>,

    /// Key-value data attached by the app's own moderation
    /// (`BotClient::update_lobby_message_moderation_metadata`)
    #[serde(default)]
    pub moderation_metadata: Option<HashMap<String, String>>,

    /// [Message flags](https://discord.com/developers/docs/resources/message#message-object-message-flags)
    pub flags: Option<u64>,

    /// The application that sent it, for messages sent by apps
    pub application_id: Option<Snowflake>,

    /// When it was edited (ISO 8601)
    pub edited_timestamp: Option<String>,
}

/// [Lobby Message Delete](https://discord.com/developers/docs/events/webhook-events#lobby-message-delete)
#[derive(Debug, Clone, Deserialize)]
pub struct LobbyMessageDelete {
    /// The deleted message's id
    pub id: Snowflake,
    /// The lobby
    pub lobby_id: Snowflake,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_lobbies() {
        let lobby: Lobby = serde_json::from_value(json!({
            "id": "5", "application_id": "1", "metadata": { "map": "dust" }, "flags": 1,
            "members": [{ "id": "6", "metadata": null, "flags": 1 }],
        }))
        .unwrap();
        assert_eq!(lobby.metadata.unwrap()["map"], "dust");
        assert_eq!(lobby.members[0].flags, 1);
        assert!(lobby.linked_channel.is_none());
    }
}
