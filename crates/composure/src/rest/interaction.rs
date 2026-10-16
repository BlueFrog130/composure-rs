//! [Interaction](https://discord.com/developers/docs/interactions/receiving-and-responding#endpoints)
//! Endpoints, authorized by the Interaction's token: no bot token needed.

use std::time::Duration;

use serde::Serialize;

use crate::types::{
    ActionRow, AllowedMentions, Embed, File, InteractionResponse, Message, MessageCallbackData,
    MessageFlags, PartialAttachment, Snowflake,
};

use super::{Body, Response, Rest, Result, Transport, route::routes};

routes! {
    CREATE_INTERACTION_RESPONSE = Post "/interactions/{interaction_id}/{interaction_token}/callback";
    GET_ORIGINAL_RESPONSE = Get "/webhooks/{webhook_id}/{webhook_token}/messages/@original";
    EDIT_ORIGINAL_RESPONSE = Patch "/webhooks/{webhook_id}/{webhook_token}/messages/@original";
    DELETE_ORIGINAL_RESPONSE = Delete "/webhooks/{webhook_id}/{webhook_token}/messages/@original";
    CREATE_FOLLOWUP_MESSAGE = Post "/webhooks/{webhook_id}/{webhook_token}";
    GET_FOLLOWUP_MESSAGE = Get "/webhooks/{webhook_id}/{webhook_token}/messages/{message_id}";
    EDIT_FOLLOWUP_MESSAGE = Patch "/webhooks/{webhook_id}/{webhook_token}/messages/{message_id}";
    DELETE_FOLLOWUP_MESSAGE = Delete "/webhooks/{webhook_id}/{webhook_token}/messages/{message_id}";
}

/// REST client authorized by one Interaction's token (valid 15 minutes): follow-ups and the
/// original response. Always available to a Command; needs no bot token.
#[derive(Debug, Clone)]
pub struct InteractionClient<T> {
    rest: Rest<T>,
    application_id: Snowflake,
    interaction_id: Snowflake,
    token: String,
}

impl<T: Transport> InteractionClient<T> {
    pub fn new(
        transport: T,
        application_id: Snowflake,
        interaction_id: Snowflake,
        token: impl Into<String>,
    ) -> Self {
        Self {
            rest: Rest::new(transport, None),
            application_id,
            interaction_id,
            token: token.into(),
        }
    }

    /// Sends requests to `base` instead of [`DISCORD_API`](super::DISCORD_API) (proxies, tests)
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

    pub fn transport(&self) -> &T {
        self.rest.transport()
    }

    /// [Create Interaction Response](https://discord.com/developers/docs/interactions/receiving-and-responding#create-interaction-response):
    /// the initial response via the callback endpoint instead of the HTTP reply. Needed to upload
    /// files with it; the Adapter then answers the HTTP request with 202 and no body.
    pub async fn create_response(
        &self,
        response: &InteractionResponse,
        files: &[File],
    ) -> Result<()> {
        let url = self.rest.url(
            CREATE_INTERACTION_RESPONSE,
            &[&self.interaction_id, &self.token],
        );
        self.rest
            .call_empty(
                CREATE_INTERACTION_RESPONSE,
                url,
                Body::WithFiles(response, files),
                None,
            )
            .await
    }

    /// [Get Original Interaction Response](https://discord.com/developers/docs/interactions/receiving-and-responding#get-original-interaction-response)
    pub async fn get_original(&self) -> Result<Response<Message>> {
        let url = self.webhook_url(GET_ORIGINAL_RESPONSE);
        self.rest
            .call(GET_ORIGINAL_RESPONSE, url, Body::NONE, None)
            .await
    }

    /// [Edit Original Interaction Response](https://discord.com/developers/docs/interactions/receiving-and-responding#edit-original-interaction-response);
    /// also how a Deferred Response is completed
    pub async fn edit_original(&self, message: &EditMessage<'_>) -> Result<Response<Message>> {
        let url = self.webhook_url(EDIT_ORIGINAL_RESPONSE);
        self.rest
            .call(
                EDIT_ORIGINAL_RESPONSE,
                url,
                Body::WithFiles(message, message.files),
                None,
            )
            .await
    }

    /// [Delete Original Interaction Response](https://discord.com/developers/docs/interactions/receiving-and-responding#delete-original-interaction-response)
    pub async fn delete_original(&self) -> Result<()> {
        let url = self.webhook_url(DELETE_ORIGINAL_RESPONSE);
        self.rest
            .call_empty(DELETE_ORIGINAL_RESPONSE, url, Body::NONE, None)
            .await
    }

    /// [Create Followup Message](https://discord.com/developers/docs/interactions/receiving-and-responding#create-followup-message)
    pub async fn create_followup(&self, message: &CreateFollowup<'_>) -> Result<Response<Message>> {
        let url = self.webhook_url(CREATE_FOLLOWUP_MESSAGE);
        self.rest
            .call(
                CREATE_FOLLOWUP_MESSAGE,
                url,
                Body::WithFiles(message, message.files),
                None,
            )
            .await
    }

    /// Shortcut for a plain-text [`create_followup`](Self::create_followup)
    pub async fn followup(&self, content: &str) -> Result<Response<Message>> {
        self.create_followup(&CreateFollowup {
            content: Some(content),
            ..Default::default()
        })
        .await
    }

    /// [Get Followup Message](https://discord.com/developers/docs/interactions/receiving-and-responding#get-followup-message)
    pub async fn get_followup(&self, message_id: Snowflake) -> Result<Response<Message>> {
        let url = self.message_url(GET_FOLLOWUP_MESSAGE, message_id);
        self.rest
            .call(GET_FOLLOWUP_MESSAGE, url, Body::NONE, None)
            .await
    }

    /// [Edit Followup Message](https://discord.com/developers/docs/interactions/receiving-and-responding#edit-followup-message)
    pub async fn edit_followup(
        &self,
        message_id: Snowflake,
        message: &EditMessage<'_>,
    ) -> Result<Response<Message>> {
        let url = self.message_url(EDIT_FOLLOWUP_MESSAGE, message_id);
        self.rest
            .call(
                EDIT_FOLLOWUP_MESSAGE,
                url,
                Body::WithFiles(message, message.files),
                None,
            )
            .await
    }

    /// [Delete Followup Message](https://discord.com/developers/docs/interactions/receiving-and-responding#delete-followup-message)
    pub async fn delete_followup(&self, message_id: Snowflake) -> Result<()> {
        let url = self.message_url(DELETE_FOLLOWUP_MESSAGE, message_id);
        self.rest
            .call_empty(DELETE_FOLLOWUP_MESSAGE, url, Body::NONE, None)
            .await
    }

    fn webhook_url(&self, route: super::Route) -> String {
        self.rest.url(route, &[&self.application_id, &self.token])
    }

    fn message_url(&self, route: super::Route, message_id: Snowflake) -> String {
        self.rest
            .url(route, &[&self.application_id, &self.token, &message_id])
    }
}

/// Body of [`InteractionClient::create_followup`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CreateFollowup<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,

    /// up to 10
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<&'a [Embed]>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<&'a AllowedMentions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<&'a [ActionRow]>,

    /// metadata for `files` (and kept attachments); generated from `files` when absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<&'a [PartialAttachment]>,

    /// only `SUPPRESS_EMBEDS`, `EPHEMERAL`, `SUPPRESS_NOTIFICATIONS` and `IS_COMPONENTS_V2` can be set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,

    #[serde(skip)]
    pub files: &'a [File],
}

/// Body of the edit Endpoints. Absent fields are left unchanged.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct EditMessage<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<&'a [Embed]>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<&'a AllowedMentions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<&'a [ActionRow]>,

    /// attachments to keep plus metadata for `files`; generated from `files` when absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<&'a [PartialAttachment]>,

    #[serde(skip)]
    pub files: &'a [File],
}

impl<'a> From<&'a MessageCallbackData> for CreateFollowup<'a> {
    fn from(data: &'a MessageCallbackData) -> Self {
        CreateFollowup {
            content: data.content.as_deref(),
            tts: data.tts,
            embeds: data.embeds.as_deref(),
            allowed_mentions: data.allowed_mentions.as_ref(),
            components: data.components.as_deref(),
            attachments: data.attachments.as_deref(),
            flags: data.flags,
            files: &data.files,
        }
    }
}

impl<'a> From<&'a MessageCallbackData> for EditMessage<'a> {
    fn from(data: &'a MessageCallbackData) -> Self {
        EditMessage {
            content: data.content.as_deref(),
            embeds: data.embeds.as_deref(),
            flags: data.flags,
            allowed_mentions: data.allowed_mentions.as_ref(),
            components: data.components.as_deref(),
            attachments: data.attachments.as_deref(),
            files: &data.files,
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::json;

    use super::super::{Method, testing::MockTransport};
    use super::*;
    use crate::types::Embed;

    fn client(transport: MockTransport) -> InteractionClient<MockTransport> {
        InteractionClient::new(
            transport,
            Snowflake::from_u64(1),
            Snowflake::from_u64(2),
            "tok",
        )
    }

    const MESSAGE: &str = r#"{"id":"10","channel_id":"11","author":{"id":"1","username":"bot","discriminator":"0","global_name":null,"avatar":null},"content":"hi","timestamp":"2024-01-01T00:00:00.000000+00:00","edited_timestamp":null,"tts":false,"mention_everyone":false,"mentions":[],"mention_roles":[],"attachments":[],"embeds":[],"pinned":false,"type":0}"#;

    #[test]
    fn followup_posts_to_webhook_without_auth() {
        let client = client(MockTransport::default().respond(200, MESSAGE));
        let transport = client.transport();
        let message = block_on(client.followup("hi")).unwrap().model().unwrap();
        assert_eq!(message.content, "hi");
        let request = transport.request(0);
        assert_eq!(request.method, Method::Post);
        assert_eq!(request.url, "https://discord.com/api/v10/webhooks/1/tok");
        assert_eq!(request.header("authorization"), None);
        assert_eq!(transport.body(0), json!({"content": "hi"}));
    }

    fn empty_client() -> InteractionClient<MockTransport> {
        InteractionClient::new(
            MockTransport::default().respond(204, ""),
            Snowflake::from_u64(1),
            Snowflake::from_u64(2),
            "tok",
        )
    }

    #[test]
    fn routes() {
        let c = empty_client();
        block_on(c.create_response(
            &InteractionResponse::DeferredChannelMessageWithSource(None),
            &[],
        ))
        .unwrap();
        block_on(c.get_original()).unwrap();
        block_on(c.edit_original(&EditMessage::default())).unwrap();
        block_on(c.delete_original()).unwrap();
        block_on(c.get_followup(Snowflake::from_u64(9))).unwrap();
        block_on(c.edit_followup(Snowflake::from_u64(9), &EditMessage::default())).unwrap();
        block_on(c.delete_followup(Snowflake::from_u64(9))).unwrap();
        let base = "https://discord.com/api/v10";
        let sent: Vec<_> = c
            .transport()
            .requests
            .borrow()
            .iter()
            .map(|r| (r.method, r.url.strip_prefix(base).unwrap().to_string()))
            .collect();
        assert_eq!(
            sent,
            [
                (Method::Post, "/interactions/2/tok/callback".to_string()),
                (
                    Method::Get,
                    "/webhooks/1/tok/messages/@original".to_string()
                ),
                (
                    Method::Patch,
                    "/webhooks/1/tok/messages/@original".to_string()
                ),
                (
                    Method::Delete,
                    "/webhooks/1/tok/messages/@original".to_string()
                ),
                (Method::Get, "/webhooks/1/tok/messages/9".to_string()),
                (Method::Patch, "/webhooks/1/tok/messages/9".to_string()),
                (Method::Delete, "/webhooks/1/tok/messages/9".to_string()),
            ]
        );
        assert_eq!(c.transport().body(0), json!({"type": 5}));
    }

    #[test]
    fn edit_from_callback_data_and_files_go_multipart() {
        let c = empty_client();
        let InteractionResponse::ChannelMessageWithSource(data) =
            InteractionResponse::respond_with_embed(Embed::default())
        else {
            unreachable!()
        };
        let files = [File::new("a.png", vec![1, 2, 3])];
        let edit = EditMessage {
            files: &files,
            ..EditMessage::from(&data)
        };
        block_on(c.edit_original(&edit)).unwrap();
        let request = c.transport().request(0);
        assert!(
            request
                .header("content-type")
                .unwrap()
                .starts_with("multipart/form-data; boundary=")
        );
        let body = String::from_utf8_lossy(request.body.as_deref().unwrap()).into_owned();
        assert!(body.contains(r#"name="files[0]"; filename="a.png""#));
        assert!(body.contains(r#""embeds":[{"#));
    }
}
