//! [Webhook Events](https://discord.com/developers/docs/events/webhook-events): notifications
//! Discord POSTs to the app's events URL.

use std::collections::HashMap;

use serde::Deserialize;
use serde_json::value::RawValue;

use crate::types::{ApplicationIntegrationType, Entitlement, PartialGuild, Snowflake, User};

/// The request body Discord sends to the events URL
#[derive(Debug, Clone, Deserialize)]
pub struct WebhookEventPayload {
    /// Always 1
    pub version: u8,

    pub application_id: Snowflake,

    /// 0 is a Ping (URL verification), 1 an event
    #[serde(rename = "type")]
    pub t: u8,

    /// Present when `t` is 1
    pub event: Option<EventBody>,
}

impl WebhookEventPayload {
    pub fn is_ping(&self) -> bool {
        self.t == 0
    }
}

/// One event, its data not yet parsed
#[derive(Debug, Clone, Deserialize)]
pub struct EventBody {
    /// Event type name, e.g. `APPLICATION_AUTHORIZED`
    #[serde(rename = "type")]
    pub kind: String,

    /// When the event happened (ISO 8601)
    pub timestamp: String,

    pub data: Option<Box<RawValue>>,
}

impl EventBody {
    /// The typed event. Never fails: event types not modeled here, and data that does not match
    /// its model, are [`Event::Unknown`], so a new or changed event never breaks a bot.
    pub fn parse(&self) -> Event {
        fn data<T: serde::de::DeserializeOwned>(body: &EventBody) -> Option<T> {
            serde_json::from_slice(body.data.as_ref()?.get().as_bytes()).ok()
        }
        let event = match self.kind.as_str() {
            "APPLICATION_AUTHORIZED" => data(self).map(Event::ApplicationAuthorized),
            "APPLICATION_DEAUTHORIZED" => data(self).map(Event::ApplicationDeauthorized),
            "ENTITLEMENT_CREATE" => data(self).map(Event::EntitlementCreate),
            "ENTITLEMENT_UPDATE" => data(self).map(Event::EntitlementUpdate),
            "ENTITLEMENT_DELETE" => data(self).map(Event::EntitlementDelete),
            "LOBBY_MESSAGE_CREATE" => data(self).map(Event::LobbyMessageCreate),
            "LOBBY_MESSAGE_UPDATE" => data(self).map(Event::LobbyMessageUpdate),
            "LOBBY_MESSAGE_DELETE" => data(self).map(Event::LobbyMessageDelete),
            _ => None,
        };
        event.unwrap_or_else(|| Event::Unknown {
            kind: self.kind.clone(),
            data: self.data.clone(),
        })
    }
}

/// A Webhook Event's typed data
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Event {
    /// The app was installed to a server or a user
    ApplicationAuthorized(ApplicationAuthorized),
    /// A user removed the app
    ApplicationDeauthorized(ApplicationDeauthorized),
    EntitlementCreate(Entitlement),
    EntitlementUpdate(Entitlement),
    EntitlementDelete(Entitlement),
    LobbyMessageCreate(LobbyMessage),
    LobbyMessageUpdate(LobbyMessage),
    LobbyMessageDelete(LobbyMessageDelete),
    /// Any other event (game DMs, quests, events added later), or one whose data did not match
    /// its model
    Unknown {
        kind: String,
        data: Option<Box<RawValue>>,
    },
}

/// [Application Authorized](https://discord.com/developers/docs/events/webhook-events#application-authorized)
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationAuthorized {
    /// Server or user install; absent for OAuth2 flows without installation
    pub integration_type: Option<ApplicationIntegrationType>,

    /// Who authorized the app
    pub user: User,

    /// OAuth2 scopes granted
    pub scopes: Vec<String>,

    /// The server, for server installs
    pub guild: Option<PartialGuild>,
}

/// [Application Deauthorized](https://discord.com/developers/docs/events/webhook-events#application-deauthorized)
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationDeauthorized {
    pub user: User,
}

/// [Lobby Message](https://discord.com/developers/docs/events/webhook-events#lobby-message-create)
#[derive(Debug, Clone, Deserialize)]
pub struct LobbyMessage {
    pub id: Snowflake,

    #[serde(rename = "type")]
    pub t: u8,

    pub content: String,

    pub lobby_id: Snowflake,

    pub channel_id: Snowflake,

    pub author: User,

    pub metadata: Option<HashMap<String, String>>,

    pub flags: Option<u64>,

    pub application_id: Option<Snowflake>,

    pub edited_timestamp: Option<String>,
}

/// [Lobby Message Delete](https://discord.com/developers/docs/events/webhook-events#lobby-message-delete)
#[derive(Debug, Clone, Deserialize)]
pub struct LobbyMessageDelete {
    pub id: Snowflake,
    pub lobby_id: Snowflake,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn event(kind: &str, data: serde_json::Value) -> Event {
        let payload: WebhookEventPayload = serde_json::from_value(json!({
            "version": 1, "application_id": "1", "type": 1,
            "event": { "type": kind, "timestamp": "2024-10-18T14:42:53.064834", "data": data },
        }))
        .unwrap();
        payload.event.unwrap().parse()
    }

    fn user() -> serde_json::Value {
        json!({ "id": "5", "username": "ana", "discriminator": "0", "global_name": null, "avatar": null })
    }

    #[test]
    fn parses_known_events() {
        let Event::ApplicationAuthorized(authorized) = event(
            "APPLICATION_AUTHORIZED",
            json!({ "integration_type": 1, "user": user(), "scopes": ["applications.commands"] }),
        ) else {
            panic!()
        };
        assert_eq!(
            authorized.integration_type,
            Some(ApplicationIntegrationType::UserInstall)
        );
        assert!(authorized.guild.is_none());

        let Event::ApplicationAuthorized(authorized) = event(
            "APPLICATION_AUTHORIZED",
            json!({ "integration_type": 0, "user": user(), "scopes": ["bot"],
                    "guild": { "id": "7", "name": "Home", "icon": null, "owner_id": "5", "features": [] } }),
        ) else {
            panic!()
        };
        assert_eq!(authorized.guild.unwrap().name, "Home");

        let Event::EntitlementCreate(entitlement) = event(
            "ENTITLEMENT_CREATE",
            json!({ "id": "9", "sku_id": "8", "application_id": "1", "user_id": "5", "type": 8,
                    "deleted": false, "starts_at": null, "ends_at": null, "consumed": false }),
        ) else {
            panic!()
        };
        assert_eq!(entitlement.kind, 8);
        assert_eq!(entitlement.user_id.unwrap().to_u64(), 5);

        assert!(matches!(
            event(
                "LOBBY_MESSAGE_DELETE",
                json!({ "id": "1", "lobby_id": "2" })
            ),
            Event::LobbyMessageDelete(_)
        ));
    }

    #[test]
    fn unknown_and_mismatched_events_are_unknown() {
        let Event::Unknown { kind, data } = event("QUEST_USER_ENROLLMENT", json!({ "x": 1 }))
        else {
            panic!()
        };
        assert_eq!(kind, "QUEST_USER_ENROLLMENT");
        assert_eq!(data.unwrap().get(), r#"{"x":1}"#);

        assert!(matches!(
            event("APPLICATION_DEAUTHORIZED", json!({ "nope": true })),
            Event::Unknown { .. }
        ));
    }

    #[test]
    fn ping_has_no_event() {
        let payload: WebhookEventPayload =
            serde_json::from_value(json!({ "version": 1, "application_id": "1", "type": 0 }))
                .unwrap();
        assert!(payload.is_ping() && payload.event.is_none());
    }
}
