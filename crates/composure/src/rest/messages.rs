//! [Messages](https://discord.com/developers/docs/resources/message#endpoints) and reactions

use serde::Serialize;

use crate::types::{
    AllowedMentions, Component, Embed, File, Message, MessageFlags, PartialAttachment,
    PinnedMessages, PollCreate, Snowflake, User,
};

use super::{
    Body, BotClient, EditMessage, Response, Result, Transport,
    route::{PathSegment, Query, routes},
};

routes! {
    CREATE_MESSAGE = Post "/channels/{channel_id}/messages";
    GET_MESSAGE = Get "/channels/{channel_id}/messages/{message_id}";
    EDIT_MESSAGE = Patch "/channels/{channel_id}/messages/{message_id}";
    DELETE_MESSAGE = Delete "/channels/{channel_id}/messages/{message_id}";
    CREATE_REACTION = Put "/channels/{channel_id}/messages/{message_id}/reactions/{emoji_name}/@me";
    DELETE_OWN_REACTION = Delete "/channels/{channel_id}/messages/{message_id}/reactions/{emoji_name}/@me";
    GET_MESSAGES = Get "/channels/{channel_id}/messages";
    BULK_DELETE_MESSAGES = Post "/channels/{channel_id}/messages/bulk-delete";
    GET_PINS = Get "/channels/{channel_id}/messages/pins";
    PIN_MESSAGE = Put "/channels/{channel_id}/messages/pins/{message_id}";
    UNPIN_MESSAGE = Delete "/channels/{channel_id}/messages/pins/{message_id}";
    CROSSPOST_MESSAGE = Post "/channels/{channel_id}/messages/{message_id}/crosspost";
    GET_REACTIONS = Get "/channels/{channel_id}/messages/{message_id}/reactions/{emoji_name}";
    DELETE_ALL_REACTIONS = Delete "/channels/{channel_id}/messages/{message_id}/reactions";
    DELETE_ALL_REACTIONS_FOR_EMOJI = Delete "/channels/{channel_id}/messages/{message_id}/reactions/{emoji_name}";
    DELETE_USER_REACTION = Delete "/channels/{channel_id}/messages/{message_id}/reactions/{emoji_name}/{user_id}";
}

/// Which messages [`BotClient::get_messages`] returns: at most one of `around`, `before`,
/// `after`; newest first otherwise
#[derive(Debug, Clone, Copy, Default)]
pub struct GetMessages {
    /// Messages around this id
    pub around: Option<Snowflake>,
    /// Messages before this id
    pub before: Option<Snowflake>,
    /// Messages after this id
    pub after: Option<Snowflake>,
    /// 1-100, default 50
    pub limit: Option<u8>,
}

#[derive(Serialize)]
struct BulkDelete<'a> {
    messages: &'a [Snowflake],
}

/// Body of [`BotClient::create_message`]; at least one of content, embeds, components, files
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CreateMessage<'a> {
    /// Text, up to 2000 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<&'a str>,

    /// Read aloud
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,

    /// up to 10
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<&'a [Embed]>,

    /// Who may be pinged
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<&'a AllowedMentions>,

    /// Components: action rows, or the whole message with `IS_COMPONENTS_V2`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<&'a [Component]>,

    /// metadata for `files`; generated from `files` when absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<&'a [PartialAttachment]>,

    /// only `SUPPRESS_EMBEDS`, `SUPPRESS_NOTIFICATIONS` and `IS_COMPONENTS_V2` can be set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,

    /// A poll sent with the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<&'a PollCreate>,

    /// Files to upload, sent as multipart
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

impl<T: Transport> BotClient<T> {
    /// [Get Channel Messages](https://discord.com/developers/docs/resources/message#get-channel-messages)
    pub async fn get_messages(
        &self,
        channel_id: Snowflake,
        query: &GetMessages,
    ) -> Result<Response<Vec<Message>>> {
        let mut url = self.rest.url(GET_MESSAGES, &[&channel_id]);
        Query::new(&mut url)
            .push_opt("around", query.around)
            .push_opt("before", query.before)
            .push_opt("after", query.after)
            .push_opt("limit", query.limit);
        self.rest.call(GET_MESSAGES, url, Body::NONE, None).await
    }

    /// [Bulk Delete Messages](https://discord.com/developers/docs/resources/message#bulk-delete-messages):
    /// 2-100 messages, none older than two weeks
    pub async fn bulk_delete_messages(
        &self,
        channel_id: Snowflake,
        messages: &[Snowflake],
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(BULK_DELETE_MESSAGES, &[&channel_id]);
        self.rest
            .call_empty(
                BULK_DELETE_MESSAGES,
                url,
                Body::Json(&BulkDelete { messages }),
                reason,
            )
            .await
    }

    /// [Get Channel Pins](https://discord.com/developers/docs/resources/message#get-channel-pins),
    /// newest first; `before` is an ISO 8601 pin time for paging
    pub async fn get_pinned_messages(
        &self,
        channel_id: Snowflake,
        before: Option<&str>,
        limit: Option<u8>,
    ) -> Result<Response<PinnedMessages>> {
        let mut url = self.rest.url(GET_PINS, &[&channel_id]);
        Query::new(&mut url)
            .push_opt("before", before)
            .push_opt("limit", limit);
        self.rest.call(GET_PINS, url, Body::NONE, None).await
    }

    /// [Pin Message](https://discord.com/developers/docs/resources/message#pin-message)
    pub async fn pin_message(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(PIN_MESSAGE, &[&channel_id, &message_id]);
        self.rest
            .call_empty(PIN_MESSAGE, url, Body::NONE, reason)
            .await
    }

    /// [Unpin Message](https://discord.com/developers/docs/resources/message#unpin-message)
    pub async fn unpin_message(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(UNPIN_MESSAGE, &[&channel_id, &message_id]);
        self.rest
            .call_empty(UNPIN_MESSAGE, url, Body::NONE, reason)
            .await
    }

    /// [Crosspost Message](https://discord.com/developers/docs/resources/message#crosspost-message)
    /// from an announcement channel to its followers
    pub async fn crosspost_message(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<Response<Message>> {
        let url = self
            .rest
            .url(CROSSPOST_MESSAGE, &[&channel_id, &message_id]);
        self.rest
            .call(CROSSPOST_MESSAGE, url, Body::NONE, None)
            .await
    }

    /// [Get Reactions](https://discord.com/developers/docs/resources/message#get-reactions):
    /// users who reacted with `emoji`, paged by `after` (1-100, default 25)
    pub async fn get_reactions(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        emoji: &str,
        after: Option<Snowflake>,
        limit: Option<u8>,
    ) -> Result<Response<Vec<User>>> {
        let mut url = self.rest.url(
            GET_REACTIONS,
            &[&channel_id, &message_id, &PathSegment(emoji)],
        );
        Query::new(&mut url)
            .push_opt("after", after)
            .push_opt("limit", limit);
        self.rest.call(GET_REACTIONS, url, Body::NONE, None).await
    }

    /// [Delete All Reactions](https://discord.com/developers/docs/resources/message#delete-all-reactions)
    pub async fn delete_all_reactions(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<()> {
        let url = self
            .rest
            .url(DELETE_ALL_REACTIONS, &[&channel_id, &message_id]);
        self.rest
            .call_empty(DELETE_ALL_REACTIONS, url, Body::NONE, None)
            .await
    }

    /// [Delete All Reactions for Emoji](https://discord.com/developers/docs/resources/message#delete-all-reactions-for-emoji)
    pub async fn delete_all_reactions_for_emoji(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        emoji: &str,
    ) -> Result<()> {
        let url = self.rest.url(
            DELETE_ALL_REACTIONS_FOR_EMOJI,
            &[&channel_id, &message_id, &PathSegment(emoji)],
        );
        self.rest
            .call_empty(DELETE_ALL_REACTIONS_FOR_EMOJI, url, Body::NONE, None)
            .await
    }

    /// [Delete User Reaction](https://discord.com/developers/docs/resources/message#delete-user-reaction)
    pub async fn delete_user_reaction(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        emoji: &str,
        user_id: Snowflake,
    ) -> Result<()> {
        let url = self.rest.url(
            DELETE_USER_REACTION,
            &[&channel_id, &message_id, &PathSegment(emoji), &user_id],
        );
        self.rest
            .call_empty(DELETE_USER_REACTION, url, Body::NONE, None)
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

    #[test]
    fn history_pins_and_reactions() {
        use super::super::Method::*;

        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        let (ch, m, u) = (
            Snowflake::from_u64(5),
            Snowflake::from_u64(6),
            Snowflake::from_u64(7),
        );
        let query = GetMessages {
            before: Some(m),
            limit: Some(10),
            ..Default::default()
        };
        block_on(c.get_messages(ch, &query)).unwrap();
        block_on(c.bulk_delete_messages(ch, &[m, u], Some("spam"))).unwrap();
        block_on(c.get_pinned_messages(ch, Some("2024-01-01T00:00:00Z"), None)).unwrap();
        block_on(c.pin_message(ch, m, None)).unwrap();
        block_on(c.unpin_message(ch, m, None)).unwrap();
        block_on(c.crosspost_message(ch, m)).unwrap();
        block_on(c.get_reactions(ch, m, "x", Some(u), Some(5))).unwrap();
        block_on(c.delete_all_reactions(ch, m)).unwrap();
        block_on(c.delete_all_reactions_for_emoji(ch, m, "x")).unwrap();
        block_on(c.delete_user_reaction(ch, m, "x", u)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/channels/5/messages?before=6&limit=10".into()),
                (Post, "/channels/5/messages/bulk-delete".into()),
                (
                    Get,
                    "/channels/5/messages/pins?before=2024-01-01T00%3A00%3A00Z".into()
                ),
                (Put, "/channels/5/messages/pins/6".into()),
                (Delete, "/channels/5/messages/pins/6".into()),
                (Post, "/channels/5/messages/6/crosspost".into()),
                (
                    Get,
                    "/channels/5/messages/6/reactions/x?after=7&limit=5".into()
                ),
                (Delete, "/channels/5/messages/6/reactions".into()),
                (Delete, "/channels/5/messages/6/reactions/x".into()),
                (Delete, "/channels/5/messages/6/reactions/x/7".into()),
            ]
        );
        assert_eq!(t.body(1), json!({ "messages": ["6", "7"] }));
    }
}
