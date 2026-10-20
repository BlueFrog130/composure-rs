//! [Webhooks](https://discord.com/developers/docs/resources/webhook#endpoints): managing them
//! with the bot token, and [`WebhookClient`] for using one with its token

use std::time::Duration;

use serde::Serialize;

use crate::types::{
    AllowedMentions, Component, Embed, File, Message, MessageFlags, PartialAttachment, PollCreate,
    Snowflake, Webhook,
};

use super::{
    Body, BotClient, EditMessage, Response, Rest, Result, Route, Transport,
    route::{Query, routes},
};

routes! {
    CREATE_WEBHOOK = Post "/channels/{channel_id}/webhooks";
    GET_CHANNEL_WEBHOOKS = Get "/channels/{channel_id}/webhooks";
    GET_GUILD_WEBHOOKS = Get "/guilds/{guild_id}/webhooks";
    GET_WEBHOOK = Get "/webhooks/{webhook_id}";
    MODIFY_WEBHOOK = Patch "/webhooks/{webhook_id}";
    DELETE_WEBHOOK = Delete "/webhooks/{webhook_id}";
    GET_WEBHOOK_WITH_TOKEN = Get "/webhooks/{webhook_id}/{webhook_token}";
    MODIFY_WEBHOOK_WITH_TOKEN = Patch "/webhooks/{webhook_id}/{webhook_token}";
    DELETE_WEBHOOK_WITH_TOKEN = Delete "/webhooks/{webhook_id}/{webhook_token}";
    EXECUTE_WEBHOOK = Post "/webhooks/{webhook_id}/{webhook_token}";
    GET_WEBHOOK_MESSAGE = Get "/webhooks/{webhook_id}/{webhook_token}/messages/{message_id}";
    EDIT_WEBHOOK_MESSAGE = Patch "/webhooks/{webhook_id}/{webhook_token}/messages/{message_id}";
    DELETE_WEBHOOK_MESSAGE = Delete "/webhooks/{webhook_id}/{webhook_token}/messages/{message_id}";
}

/// Body of [`BotClient::create_webhook`], [`BotClient::modify_webhook`] and
/// [`WebhookClient::modify`] (which cannot move channels)
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct WebhookFields<'a> {
    /// 1-80 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,

    /// Data URI; `Some(None)` removes it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Option<&'a str>>,

    /// Modify only: move to this channel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Body of [`WebhookClient::execute`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ExecuteWebhook<'a> {
    /// Text, up to 2000 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<&'a str>,

    /// Overrides the webhook's name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<&'a str>,

    /// Overrides the webhook's avatar
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<&'a str>,

    /// Read aloud
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,

    /// Up to 10 embeds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<&'a [Embed]>,

    /// Who may be pinged
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<&'a AllowedMentions>,

    /// Only for webhooks owned by an application, unless non-interactive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<&'a [Component]>,

    /// Metadata of `files`; generated from them when absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<&'a [PartialAttachment]>,

    /// Only `SUPPRESS_EMBEDS`, `SUPPRESS_NOTIFICATIONS` and `IS_COMPONENTS_V2` can be set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,

    /// Forum/media channel webhooks: create a post with this name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_name: Option<&'a str>,

    /// Forum/media channel posts: tags to apply
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_tags: Option<&'a [Snowflake]>,

    /// A poll sent with the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<&'a PollCreate>,

    /// Files to upload, sent as multipart
    #[serde(skip)]
    pub files: &'a [File],

    /// Wait for the message to be created and return it (otherwise 204, no body)
    #[serde(skip)]
    pub wait: bool,

    /// Send to this thread of the webhook's channel
    #[serde(skip)]
    pub thread_id: Option<Snowflake>,
}

impl<T: Transport> BotClient<T> {
    /// [Create Webhook](https://discord.com/developers/docs/resources/webhook#create-webhook)
    /// (`name` required)
    pub async fn create_webhook(
        &self,
        channel_id: Snowflake,
        webhook: &WebhookFields<'_>,
    ) -> Result<Response<Webhook>> {
        let url = self.rest.url(CREATE_WEBHOOK, &[&channel_id]);
        self.rest
            .call(CREATE_WEBHOOK, url, Body::Json(webhook), webhook.reason)
            .await
    }

    /// [Get Channel Webhooks](https://discord.com/developers/docs/resources/webhook#get-channel-webhooks)
    pub async fn get_channel_webhooks(
        &self,
        channel_id: Snowflake,
    ) -> Result<Response<Vec<Webhook>>> {
        let url = self.rest.url(GET_CHANNEL_WEBHOOKS, &[&channel_id]);
        self.rest
            .call(GET_CHANNEL_WEBHOOKS, url, Body::NONE, None)
            .await
    }

    /// [Get Guild Webhooks](https://discord.com/developers/docs/resources/webhook#get-guild-webhooks)
    pub async fn get_guild_webhooks(&self, guild_id: Snowflake) -> Result<Response<Vec<Webhook>>> {
        let url = self.rest.url(GET_GUILD_WEBHOOKS, &[&guild_id]);
        self.rest
            .call(GET_GUILD_WEBHOOKS, url, Body::NONE, None)
            .await
    }

    /// [Get Webhook](https://discord.com/developers/docs/resources/webhook#get-webhook)
    pub async fn get_webhook(&self, webhook_id: Snowflake) -> Result<Response<Webhook>> {
        let url = self.rest.url(GET_WEBHOOK, &[&webhook_id]);
        self.rest.call(GET_WEBHOOK, url, Body::NONE, None).await
    }

    /// [Modify Webhook](https://discord.com/developers/docs/resources/webhook#modify-webhook)
    pub async fn modify_webhook(
        &self,
        webhook_id: Snowflake,
        webhook: &WebhookFields<'_>,
    ) -> Result<Response<Webhook>> {
        let url = self.rest.url(MODIFY_WEBHOOK, &[&webhook_id]);
        self.rest
            .call(MODIFY_WEBHOOK, url, Body::Json(webhook), webhook.reason)
            .await
    }

    /// [Delete Webhook](https://discord.com/developers/docs/resources/webhook#delete-webhook)
    pub async fn delete_webhook(&self, webhook_id: Snowflake, reason: Option<&str>) -> Result<()> {
        let url = self.rest.url(DELETE_WEBHOOK, &[&webhook_id]);
        self.rest
            .call_empty(DELETE_WEBHOOK, url, Body::NONE, reason)
            .await
    }
}

/// REST client for one webhook, authorized by its token (no bot token needed): execute it and
/// manage its messages
#[derive(Debug, Clone)]
pub struct WebhookClient<T> {
    rest: Rest<T>,
    id: Snowflake,
    token: String,
}

impl<T: Transport> WebhookClient<T> {
    /// A client for webhook `id`, authorized by its `token`
    pub fn new(transport: T, id: Snowflake, token: impl Into<String>) -> Self {
        Self {
            rest: Rest::new(transport, None),
            id,
            token: token.into(),
        }
    }

    /// From a webhook URL (`https://discord.com/api/webhooks/<id>/<token>`); `None` if it is not
    /// one
    pub fn from_url(transport: T, url: &str) -> Option<Self> {
        let mut parts = url.trim_end_matches('/').rsplit('/');
        let token = parts.next()?;
        let id = parts.next()?.parse().ok()?;
        (parts.next()? == "webhooks").then(|| Self::new(transport, id, token))
    }

    /// Sends requests to `base` instead of [`DISCORD_API`](super::DISCORD_API)
    pub fn with_base_url(mut self, base: impl Into<String>) -> Self {
        self.rest.set_base(base.into());
        self
    }

    /// Longest single rate-limit wait before giving up with
    /// [`Error::RateLimited`](super::Error::RateLimited)
    pub fn with_max_rate_limit_wait(mut self, wait: Duration) -> Self {
        self.rest.set_max_rate_limit_wait(wait);
        self
    }

    /// The Transport requests go through
    pub fn transport(&self) -> &T {
        self.rest.transport()
    }

    fn url(&self, route: Route) -> String {
        self.rest.url(route, &[&self.id, &self.token])
    }

    fn message_url(
        &self,
        route: Route,
        message_id: Snowflake,
        thread_id: Option<Snowflake>,
    ) -> String {
        let mut url = self.rest.url(route, &[&self.id, &self.token, &message_id]);
        Query::new(&mut url).push_opt("thread_id", thread_id);
        url
    }

    /// [Get Webhook with Token](https://discord.com/developers/docs/resources/webhook#get-webhook-with-token)
    pub async fn get(&self) -> Result<Response<Webhook>> {
        let url = self.url(GET_WEBHOOK_WITH_TOKEN);
        self.rest
            .call(GET_WEBHOOK_WITH_TOKEN, url, Body::NONE, None)
            .await
    }

    /// [Modify Webhook with Token](https://discord.com/developers/docs/resources/webhook#modify-webhook-with-token)
    pub async fn modify(&self, webhook: &WebhookFields<'_>) -> Result<Response<Webhook>> {
        let url = self.url(MODIFY_WEBHOOK_WITH_TOKEN);
        let webhook = WebhookFields {
            channel_id: None,
            ..*webhook
        };
        self.rest
            .call(
                MODIFY_WEBHOOK_WITH_TOKEN,
                url,
                Body::Json(&webhook),
                webhook.reason,
            )
            .await
    }

    /// [Delete Webhook with Token](https://discord.com/developers/docs/resources/webhook#delete-webhook-with-token)
    pub async fn delete(&self, reason: Option<&str>) -> Result<()> {
        let url = self.url(DELETE_WEBHOOK_WITH_TOKEN);
        self.rest
            .call_empty(DELETE_WEBHOOK_WITH_TOKEN, url, Body::NONE, reason)
            .await
    }

    /// [Execute Webhook](https://discord.com/developers/docs/resources/webhook#execute-webhook).
    /// With `wait`, the created message; without, status 204 and no body.
    pub async fn execute(&self, message: &ExecuteWebhook<'_>) -> Result<Response<Message>> {
        let mut url = self.url(EXECUTE_WEBHOOK);
        Query::new(&mut url)
            .push_opt("wait", message.wait.then_some(true))
            .push_opt("thread_id", message.thread_id);
        self.rest
            .call(
                EXECUTE_WEBHOOK,
                url,
                Body::WithFiles(message, message.files),
                None,
            )
            .await
    }

    /// [Get Webhook Message](https://discord.com/developers/docs/resources/webhook#get-webhook-message)
    pub async fn get_message(
        &self,
        message_id: Snowflake,
        thread_id: Option<Snowflake>,
    ) -> Result<Response<Message>> {
        let url = self.message_url(GET_WEBHOOK_MESSAGE, message_id, thread_id);
        self.rest
            .call(GET_WEBHOOK_MESSAGE, url, Body::NONE, None)
            .await
    }

    /// [Edit Webhook Message](https://discord.com/developers/docs/resources/webhook#edit-webhook-message)
    pub async fn edit_message(
        &self,
        message_id: Snowflake,
        thread_id: Option<Snowflake>,
        message: &EditMessage<'_>,
    ) -> Result<Response<Message>> {
        let url = self.message_url(EDIT_WEBHOOK_MESSAGE, message_id, thread_id);
        self.rest
            .call(
                EDIT_WEBHOOK_MESSAGE,
                url,
                Body::WithFiles(message, message.files),
                None,
            )
            .await
    }

    /// [Delete Webhook Message](https://discord.com/developers/docs/resources/webhook#delete-webhook-message)
    pub async fn delete_message(
        &self,
        message_id: Snowflake,
        thread_id: Option<Snowflake>,
    ) -> Result<()> {
        let url = self.message_url(DELETE_WEBHOOK_MESSAGE, message_id, thread_id);
        self.rest
            .call_empty(DELETE_WEBHOOK_MESSAGE, url, Body::NONE, None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::json;

    use super::super::{Method::*, testing::MockTransport};
    use super::*;

    #[test]
    fn bot_routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        let id = Snowflake::from_u64(5);
        let fields = WebhookFields {
            name: Some("Alerts"),
            avatar: Some(None),
            ..Default::default()
        };
        block_on(c.create_webhook(id, &fields)).unwrap();
        block_on(c.get_channel_webhooks(id)).unwrap();
        block_on(c.get_guild_webhooks(id)).unwrap();
        block_on(c.get_webhook(id)).unwrap();
        block_on(c.modify_webhook(id, &fields)).unwrap();
        block_on(c.delete_webhook(id, Some("unused"))).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Post, "/channels/5/webhooks".into()),
                (Get, "/channels/5/webhooks".into()),
                (Get, "/guilds/5/webhooks".into()),
                (Get, "/webhooks/5".into()),
                (Patch, "/webhooks/5".into()),
                (Delete, "/webhooks/5".into()),
            ]
        );
        assert_eq!(t.body(0), json!({ "name": "Alerts", "avatar": null }));
    }

    #[test]
    fn token_routes() {
        let w = WebhookClient::from_url(
            MockTransport::default(),
            "https://discord.com/api/webhooks/5/tok",
        )
        .unwrap();
        let (m, thread) = (Snowflake::from_u64(6), Snowflake::from_u64(7));
        block_on(w.get()).unwrap();
        block_on(w.modify(&WebhookFields {
            name: Some("x"),
            channel_id: Some(thread),
            ..Default::default()
        }))
        .unwrap();
        block_on(w.execute(&ExecuteWebhook {
            content: Some("hi"),
            username: Some("Bot"),
            wait: true,
            thread_id: Some(thread),
            ..Default::default()
        }))
        .unwrap();
        block_on(w.get_message(m, None)).unwrap();
        block_on(w.edit_message(m, Some(thread), &EditMessage::default())).unwrap();
        block_on(w.delete_message(m, None)).unwrap();
        block_on(w.delete(None)).unwrap();
        let t = w.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/webhooks/5/tok".into()),
                (Patch, "/webhooks/5/tok".into()),
                (Post, "/webhooks/5/tok?wait=true&thread_id=7".into()),
                (Get, "/webhooks/5/tok/messages/6".into()),
                (Patch, "/webhooks/5/tok/messages/6?thread_id=7".into()),
                (Delete, "/webhooks/5/tok/messages/6".into()),
                (Delete, "/webhooks/5/tok".into()),
            ]
        );
        assert_eq!(
            t.body(1),
            json!({ "name": "x" }),
            "cannot move channels by token"
        );
        assert_eq!(t.body(2), json!({ "content": "hi", "username": "Bot" }));
        assert_eq!(t.request(0).header("authorization"), None);
        assert!(
            WebhookClient::from_url(MockTransport::default(), "https://example.com/x").is_none()
        );
    }
}
