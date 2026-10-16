//! [Application](https://discord.com/developers/docs/resources/application#get-current-application):
//! the app's own settings

use serde::Serialize;

use crate::types::{Application, ApplicationFlags};

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    GET_CURRENT_APPLICATION = Get "/applications/@me";
    EDIT_CURRENT_APPLICATION = Patch "/applications/@me";
}

/// Body of [`BotClient::edit_current_application`]. Absent fields are unchanged; `Some(None)`
/// clears. Images are data URIs.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct EditApplication<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<Option<&'a str>>,

    /// Only the gateway intent and limited flags can be changed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<ApplicationFlags>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactions_endpoint_url: Option<Option<&'a str>>,

    /// Up to 5
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<&'a [&'a str]>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_install_url: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_connections_verification_url: Option<Option<&'a str>>,

    /// Where Webhook Events go
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_webhooks_url: Option<Option<&'a str>>,

    /// 1 disabled, 2 enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_webhooks_status: Option<u8>,

    /// Webhook Event types to receive, e.g. `APPLICATION_AUTHORIZED`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_webhooks_types: Option<&'a [&'a str]>,
}

impl<T: Transport> BotClient<T> {
    /// [Get Current Application](https://discord.com/developers/docs/resources/application#get-current-application)
    pub async fn get_current_application(&self) -> Result<Response<Application>> {
        let url = self.rest.url(GET_CURRENT_APPLICATION, &[]);
        self.rest
            .call(GET_CURRENT_APPLICATION, url, Body::NONE, None)
            .await
    }

    /// [Edit Current Application](https://discord.com/developers/docs/resources/application#edit-current-application)
    pub async fn edit_current_application(
        &self,
        application: &EditApplication<'_>,
    ) -> Result<Response<Application>> {
        let url = self.rest.url(EDIT_CURRENT_APPLICATION, &[]);
        self.rest
            .call(EDIT_CURRENT_APPLICATION, url, Body::Json(application), None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::json;

    use super::super::{Method::*, testing::MockTransport};
    use super::*;
    use crate::types::Snowflake;

    #[test]
    fn routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        block_on(c.get_current_application()).unwrap();
        block_on(c.edit_current_application(&EditApplication {
            event_webhooks_url: Some(Some("https://bot.example/events")),
            event_webhooks_status: Some(2),
            event_webhooks_types: Some(&["APPLICATION_AUTHORIZED"]),
            ..Default::default()
        }))
        .unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/applications/@me".into()),
                (Patch, "/applications/@me".into()),
            ]
        );
        assert_eq!(
            t.body(1),
            json!({
                "event_webhooks_url": "https://bot.example/events",
                "event_webhooks_status": 2,
                "event_webhooks_types": ["APPLICATION_AUTHORIZED"],
            })
        );
    }

    #[test]
    fn current_application_deserializes() {
        let app: Application = serde_json::from_value(json!({
            "id": "1", "name": "Bot", "icon": null, "description": "", "flags": 0,
            "verify_key": "abc", "bot_public": true, "bot_require_code_grant": false,
            "interactions_endpoint_url": "https://bot.example", "approximate_guild_count": 3,
            "redirect_uris": [], "role_connections_verification_url": null,
            "owner": { "id": "2", "username": "me", "discriminator": "0", "avatar": null },
            "approximate_user_install_count": 7, "explicit_content_filter": 0, "team": null,
        }))
        .unwrap();
        assert_eq!(app.approximate_guild_count, Some(3));
        // Partial applications on messages lack most fields
        let partial: Application = serde_json::from_value(json!({
            "id": "1", "name": "Game", "icon": null, "description": "A game",
        }))
        .unwrap();
        assert!(partial.verify_key.is_empty());
    }
}
