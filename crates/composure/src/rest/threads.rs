//! [Threads](https://discord.com/developers/docs/resources/channel#start-thread-from-message):
//! starting, membership, listing

use serde::Serialize;

use crate::types::{Channel, ChannelType, Snowflake, ThreadList, ThreadMember};

use super::{
    Body, BotClient, CreateMessage, Response, Result, Transport,
    route::{Query, routes},
};

routes! {
    START_THREAD_FROM_MESSAGE = Post "/channels/{channel_id}/messages/{message_id}/threads";
    START_THREAD = Post "/channels/{channel_id}/threads";
    JOIN_THREAD = Put "/channels/{channel_id}/thread-members/@me";
    LEAVE_THREAD = Delete "/channels/{channel_id}/thread-members/@me";
    ADD_THREAD_MEMBER = Put "/channels/{channel_id}/thread-members/{user_id}";
    REMOVE_THREAD_MEMBER = Delete "/channels/{channel_id}/thread-members/{user_id}";
    GET_THREAD_MEMBER = Get "/channels/{channel_id}/thread-members/{user_id}";
    LIST_THREAD_MEMBERS = Get "/channels/{channel_id}/thread-members";
    LIST_ACTIVE_GUILD_THREADS = Get "/guilds/{guild_id}/threads/active";
    LIST_PUBLIC_ARCHIVED_THREADS = Get "/channels/{channel_id}/threads/archived/public";
    LIST_PRIVATE_ARCHIVED_THREADS = Get "/channels/{channel_id}/threads/archived/private";
    LIST_JOINED_PRIVATE_ARCHIVED_THREADS = Get "/channels/{channel_id}/users/@me/threads/archived/private";
}

/// Body of [`BotClient::start_thread`] and [`BotClient::start_thread_from_message`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct StartThread<'a> {
    /// 1-100 characters
    pub name: &'a str,

    /// Minutes of inactivity before archiving: 60, 1440, 4320, or 10080
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u32>,

    /// Without a message: public (default) or private thread
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<ChannelType>,

    /// Private threads: non-moderators can add members
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitable: Option<bool>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Body of [`BotClient::start_forum_thread`]: a forum or media post with its first message
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct StartForumThread<'a> {
    pub name: &'a str,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u32>,

    /// Forum tag ids
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_tags: Option<&'a [Snowflake]>,

    /// The first message; its `files` are uploaded with the thread
    pub message: CreateMessage<'a>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Paging of the archived thread lists: threads archived before an ISO 8601 time (or a thread id
/// for joined private threads)
#[derive(Debug, Clone, Copy, Default)]
pub struct ArchivedThreads<'a> {
    pub before: Option<&'a str>,
    pub limit: Option<u8>,
}

impl<T: Transport> BotClient<T> {
    /// [Start Thread from Message](https://discord.com/developers/docs/resources/channel#start-thread-from-message)
    pub async fn start_thread_from_message(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        thread: &StartThread<'_>,
    ) -> Result<Response<Channel>> {
        let url = self
            .rest
            .url(START_THREAD_FROM_MESSAGE, &[&channel_id, &message_id]);
        self.rest
            .call(
                START_THREAD_FROM_MESSAGE,
                url,
                Body::Json(thread),
                thread.reason,
            )
            .await
    }

    /// [Start Thread without Message](https://discord.com/developers/docs/resources/channel#start-thread-without-message)
    pub async fn start_thread(
        &self,
        channel_id: Snowflake,
        thread: &StartThread<'_>,
    ) -> Result<Response<Channel>> {
        let url = self.rest.url(START_THREAD, &[&channel_id]);
        self.rest
            .call(START_THREAD, url, Body::Json(thread), thread.reason)
            .await
    }

    /// [Start Thread in Forum or Media Channel](https://discord.com/developers/docs/resources/channel#start-thread-in-forum-or-media-channel)
    pub async fn start_forum_thread(
        &self,
        channel_id: Snowflake,
        thread: &StartForumThread<'_>,
    ) -> Result<Response<Channel>> {
        let url = self.rest.url(START_THREAD, &[&channel_id]);
        self.rest
            .call(
                START_THREAD,
                url,
                Body::WithFiles(thread, thread.message.files),
                thread.reason,
            )
            .await
    }

    /// [Join Thread](https://discord.com/developers/docs/resources/channel#join-thread)
    pub async fn join_thread(&self, channel_id: Snowflake) -> Result<()> {
        let url = self.rest.url(JOIN_THREAD, &[&channel_id]);
        self.rest
            .call_empty(JOIN_THREAD, url, Body::NONE, None)
            .await
    }

    /// [Leave Thread](https://discord.com/developers/docs/resources/channel#leave-thread)
    pub async fn leave_thread(&self, channel_id: Snowflake) -> Result<()> {
        let url = self.rest.url(LEAVE_THREAD, &[&channel_id]);
        self.rest
            .call_empty(LEAVE_THREAD, url, Body::NONE, None)
            .await
    }

    /// [Add Thread Member](https://discord.com/developers/docs/resources/channel#add-thread-member)
    pub async fn add_thread_member(&self, channel_id: Snowflake, user_id: Snowflake) -> Result<()> {
        let url = self.rest.url(ADD_THREAD_MEMBER, &[&channel_id, &user_id]);
        self.rest
            .call_empty(ADD_THREAD_MEMBER, url, Body::NONE, None)
            .await
    }

    /// [Remove Thread Member](https://discord.com/developers/docs/resources/channel#remove-thread-member)
    pub async fn remove_thread_member(
        &self,
        channel_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<()> {
        let url = self
            .rest
            .url(REMOVE_THREAD_MEMBER, &[&channel_id, &user_id]);
        self.rest
            .call_empty(REMOVE_THREAD_MEMBER, url, Body::NONE, None)
            .await
    }

    /// [Get Thread Member](https://discord.com/developers/docs/resources/channel#get-thread-member);
    /// `with_member` includes the guild member
    pub async fn get_thread_member(
        &self,
        channel_id: Snowflake,
        user_id: Snowflake,
        with_member: bool,
    ) -> Result<Response<ThreadMember>> {
        let mut url = self.rest.url(GET_THREAD_MEMBER, &[&channel_id, &user_id]);
        Query::new(&mut url).push_opt("with_member", with_member.then_some(true));
        self.rest
            .call(GET_THREAD_MEMBER, url, Body::NONE, None)
            .await
    }

    /// [List Thread Members](https://discord.com/developers/docs/resources/channel#list-thread-members),
    /// paged by user id (1-100, default 100)
    pub async fn list_thread_members(
        &self,
        channel_id: Snowflake,
        with_member: bool,
        after: Option<Snowflake>,
        limit: Option<u8>,
    ) -> Result<Response<Vec<ThreadMember>>> {
        let mut url = self.rest.url(LIST_THREAD_MEMBERS, &[&channel_id]);
        Query::new(&mut url)
            .push_opt("with_member", with_member.then_some(true))
            .push_opt("after", after)
            .push_opt("limit", limit);
        self.rest
            .call(LIST_THREAD_MEMBERS, url, Body::NONE, None)
            .await
    }

    /// [List Active Guild Threads](https://discord.com/developers/docs/resources/guild#list-active-guild-threads)
    pub async fn list_active_guild_threads(
        &self,
        guild_id: Snowflake,
    ) -> Result<Response<ThreadList>> {
        let url = self.rest.url(LIST_ACTIVE_GUILD_THREADS, &[&guild_id]);
        self.rest
            .call(LIST_ACTIVE_GUILD_THREADS, url, Body::NONE, None)
            .await
    }

    /// [List Public Archived Threads](https://discord.com/developers/docs/resources/channel#list-public-archived-threads)
    pub async fn list_public_archived_threads(
        &self,
        channel_id: Snowflake,
        page: &ArchivedThreads<'_>,
    ) -> Result<Response<ThreadList>> {
        self.archived(LIST_PUBLIC_ARCHIVED_THREADS, channel_id, page)
            .await
    }

    /// [List Private Archived Threads](https://discord.com/developers/docs/resources/channel#list-private-archived-threads)
    pub async fn list_private_archived_threads(
        &self,
        channel_id: Snowflake,
        page: &ArchivedThreads<'_>,
    ) -> Result<Response<ThreadList>> {
        self.archived(LIST_PRIVATE_ARCHIVED_THREADS, channel_id, page)
            .await
    }

    /// [List Joined Private Archived Threads](https://discord.com/developers/docs/resources/channel#list-joined-private-archived-threads);
    /// `before` is a thread id here
    pub async fn list_joined_private_archived_threads(
        &self,
        channel_id: Snowflake,
        page: &ArchivedThreads<'_>,
    ) -> Result<Response<ThreadList>> {
        self.archived(LIST_JOINED_PRIVATE_ARCHIVED_THREADS, channel_id, page)
            .await
    }

    async fn archived(
        &self,
        route: super::Route,
        channel_id: Snowflake,
        page: &ArchivedThreads<'_>,
    ) -> Result<Response<ThreadList>> {
        let mut url = self.rest.url(route, &[&channel_id]);
        Query::new(&mut url)
            .push_opt("before", page.before)
            .push_opt("limit", page.limit);
        self.rest.call(route, url, Body::NONE, None).await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::json;

    use super::super::{Method::*, testing::MockTransport};
    use super::*;
    use crate::types::File;

    #[test]
    fn routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        let (ch, m, u) = (
            Snowflake::from_u64(5),
            Snowflake::from_u64(6),
            Snowflake::from_u64(7),
        );
        let thread = StartThread {
            name: "talk",
            kind: Some(ChannelType::PrivateThread),
            invitable: Some(false),
            ..Default::default()
        };
        let files = [File::new("a.txt", b"hi".to_vec())];
        let post = StartForumThread {
            name: "Bug",
            applied_tags: Some(&[m]),
            message: CreateMessage {
                content: Some("it broke"),
                files: &files,
                ..Default::default()
            },
            ..Default::default()
        };
        let page = ArchivedThreads {
            before: Some("2024-01-01T00:00:00Z"),
            limit: Some(5),
        };
        block_on(c.start_thread_from_message(ch, m, &thread)).unwrap();
        block_on(c.start_thread(ch, &thread)).unwrap();
        block_on(c.start_forum_thread(ch, &post)).unwrap();
        block_on(c.join_thread(ch)).unwrap();
        block_on(c.leave_thread(ch)).unwrap();
        block_on(c.add_thread_member(ch, u)).unwrap();
        block_on(c.remove_thread_member(ch, u)).unwrap();
        block_on(c.get_thread_member(ch, u, true)).unwrap();
        block_on(c.list_thread_members(ch, false, Some(u), Some(50))).unwrap();
        block_on(c.list_active_guild_threads(m)).unwrap();
        block_on(c.list_public_archived_threads(ch, &page)).unwrap();
        block_on(c.list_private_archived_threads(ch, &ArchivedThreads::default())).unwrap();
        block_on(c.list_joined_private_archived_threads(ch, &ArchivedThreads::default())).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Post, "/channels/5/messages/6/threads".into()),
                (Post, "/channels/5/threads".into()),
                (Post, "/channels/5/threads".into()),
                (Put, "/channels/5/thread-members/@me".into()),
                (Delete, "/channels/5/thread-members/@me".into()),
                (Put, "/channels/5/thread-members/7".into()),
                (Delete, "/channels/5/thread-members/7".into()),
                (Get, "/channels/5/thread-members/7?with_member=true".into()),
                (Get, "/channels/5/thread-members?after=7&limit=50".into()),
                (Get, "/guilds/6/threads/active".into()),
                (
                    Get,
                    "/channels/5/threads/archived/public?before=2024-01-01T00%3A00%3A00Z&limit=5"
                        .into()
                ),
                (Get, "/channels/5/threads/archived/private".into()),
                (Get, "/channels/5/users/@me/threads/archived/private".into()),
            ]
        );
        assert_eq!(
            t.body(1),
            json!({ "name": "talk", "type": 12, "invitable": false })
        );
        let forum = t.request(2);
        assert!(
            forum
                .header("content-type")
                .unwrap()
                .starts_with("multipart/form-data")
        );
        let body = String::from_utf8_lossy(forum.body.as_deref().unwrap()).into_owned();
        assert!(
            body.contains(r#""message":{"content":"it broke"}"#),
            "{body}"
        );
        assert!(body.contains(r#""applied_tags":["6"]"#));
    }
}
