//! [Messages](https://discord.com/developers/docs/resources/message#endpoints) and reactions

use serde::Serialize;

use crate::types::{
    ActionRow, AllowedMentions, Embed, File, Message, MessageFlags, PartialAttachment, Snowflake,
};

use super::{
    Body, BotClient, EditMessage, Response, Result, Transport,
    route::{PathSegment, routes},
};

routes! {
    CREATE_MESSAGE = Post "/channels/{channel_id}/messages";
    GET_MESSAGE = Get "/channels/{channel_id}/messages/{message_id}";
    EDIT_MESSAGE = Patch "/channels/{channel_id}/messages/{message_id}";
    DELETE_MESSAGE = Delete "/channels/{channel_id}/messages/{message_id}";
    CREATE_REACTION = Put "/channels/{channel_id}/messages/{message_id}/reactions/{emoji_name}/@me";
    DELETE_OWN_REACTION = Delete "/channels/{channel_id}/messages/{message_id}/reactions/{emoji_name}/@me";
}

/// Body of [`BotClient::create_message`]; at least one of content, embeds, components, files
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CreateMessage<'a> {
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

    /// metadata for `files`; generated from `files` when absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<&'a [PartialAttachment]>,

    /// only `SUPPRESS_EMBEDS`, `SUPPRESS_NOTIFICATIONS` and `IS_COMPONENTS_V2` can be set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,

    #[serde(skip)]
    pub files: &'a [File],
}

impl<T: Transport> BotClient<T> {
    /// [Create Message](https://discord.com/developers/docs/resources/message#create-message)
    pub async fn create_message(
        &self,
        channel_id: Snowflake,
        message: &CreateMessage<'_>,
    ) -> Result<Response<Message>> {
        let url = self.rest.url(CREATE_MESSAGE, &[&channel_id]);
        self.rest
            .call(
                CREATE_MESSAGE,
                url,
                Body::WithFiles(message, message.files),
                None,
            )
            .await
    }

    /// Shortcut for a plain-text [`create_message`](Self::create_message)
    pub async fn send_message(
        &self,
        channel_id: Snowflake,
        content: &str,
    ) -> Result<Response<Message>> {
        self.create_message(
            channel_id,
            &CreateMessage {
                content: Some(content),
                ..Default::default()
            },
        )
        .await
    }

    /// [Get Channel Message](https://discord.com/developers/docs/resources/message#get-channel-message)
    pub async fn get_message(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<Response<Message>> {
        let url = self.rest.url(GET_MESSAGE, &[&channel_id, &message_id]);
        self.rest.call(GET_MESSAGE, url, Body::NONE, None).await
    }

    /// [Edit Message](https://discord.com/developers/docs/resources/message#edit-message)
    pub async fn edit_message(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        message: &EditMessage<'_>,
    ) -> Result<Response<Message>> {
        let url = self.rest.url(EDIT_MESSAGE, &[&channel_id, &message_id]);
        self.rest
            .call(
                EDIT_MESSAGE,
                url,
                Body::WithFiles(message, message.files),
                None,
            )
            .await
    }

    /// [Delete Message](https://discord.com/developers/docs/resources/message#delete-message)
    pub async fn delete_message(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(DELETE_MESSAGE, &[&channel_id, &message_id]);
        self.rest
            .call_empty(DELETE_MESSAGE, url, Body::NONE, reason)
            .await
    }

    /// [Create Reaction](https://discord.com/developers/docs/resources/message#create-reaction).
    /// `emoji` is a Unicode emoji or a custom one as `name:id`.
    pub async fn create_reaction(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        emoji: &str,
    ) -> Result<()> {
        let url = self.rest.url(
            CREATE_REACTION,
            &[&channel_id, &message_id, &PathSegment(emoji)],
        );
        self.rest
            .call_empty(CREATE_REACTION, url, Body::NONE, None)
            .await
    }

    /// [Delete Own Reaction](https://discord.com/developers/docs/resources/message#delete-own-reaction)
    pub async fn delete_own_reaction(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        emoji: &str,
    ) -> Result<()> {
        let url = self.rest.url(
            DELETE_OWN_REACTION,
            &[&channel_id, &message_id, &PathSegment(emoji)],
        );
        self.rest
            .call_empty(DELETE_OWN_REACTION, url, Body::NONE, None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::json;

    use super::super::{Method, testing::MockTransport};
    use super::*;

    #[test]
    fn routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        let (channel, message) = (Snowflake::from_u64(5), Snowflake::from_u64(6));
        block_on(c.send_message(channel, "hi")).unwrap();
        block_on(c.get_message(channel, message)).unwrap();
        block_on(c.edit_message(channel, message, &EditMessage::default())).unwrap();
        block_on(c.delete_message(channel, message, Some("spam"))).unwrap();
        block_on(c.create_reaction(channel, message, "\u{1F44D}")).unwrap();
        block_on(c.delete_own_reaction(channel, message, "blob:7")).unwrap();
        let sent: Vec<_> = c
            .transport()
            .requests
            .borrow()
            .iter()
            .map(|r| (r.method, r.url.rsplit("v10").next().unwrap().to_string()))
            .collect();
        assert_eq!(
            sent,
            [
                (Method::Post, "/channels/5/messages".into()),
                (Method::Get, "/channels/5/messages/6".into()),
                (Method::Patch, "/channels/5/messages/6".into()),
                (Method::Delete, "/channels/5/messages/6".into()),
                (
                    Method::Put,
                    "/channels/5/messages/6/reactions/%F0%9F%91%8D/@me".into()
                ),
                (
                    Method::Delete,
                    "/channels/5/messages/6/reactions/blob:7/@me".into()
                ),
            ]
        );
        assert_eq!(c.transport().body(0), json!({ "content": "hi" }));
        assert_eq!(
            c.transport().request(3).header("x-audit-log-reason"),
            Some("spam")
        );
        assert_eq!(
            c.transport().request(0).header("authorization"),
            Some("Bot t")
        );
    }
}
