//! [Channels](https://discord.com/developers/docs/resources/channel#endpoints): settings,
//! permission overwrites, typing, following

use serde::Serialize;

use crate::types::{
    Channel, ChannelFlags, ChannelType, FollowedChannel, Overwrite, OverwriteType, Permissions,
    Snowflake, VideoQualityMode,
};

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    GET_CHANNEL = Get "/channels/{channel_id}";
    MODIFY_CHANNEL = Patch "/channels/{channel_id}";
    DELETE_CHANNEL = Delete "/channels/{channel_id}";
    EDIT_CHANNEL_PERMISSIONS = Put "/channels/{channel_id}/permissions/{overwrite_id}";
    DELETE_CHANNEL_PERMISSION = Delete "/channels/{channel_id}/permissions/{overwrite_id}";
    TRIGGER_TYPING = Post "/channels/{channel_id}/typing";
    FOLLOW_ANNOUNCEMENT_CHANNEL = Post "/channels/{channel_id}/followers";
}

/// Body of [`BotClient::modify_channel`]. Absent fields are unchanged; `Some(None)` clears a
/// nullable one. Thread-only fields apply to threads.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModifyChannel<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,

    /// Only text <-> announcement
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<ChannelType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Option<i32>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,

    /// Slowmode in seconds (0-21600)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<Option<u32>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<Option<u32>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<Option<u32>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_overwrites: Option<&'a [Overwrite]>,

    /// Category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Option<Snowflake>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_region: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<VideoQualityMode>,

    /// Minutes: 60, 1440, 4320, or 10080
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<ChannelFlags>,

    /// Thread: archive or unarchive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,

    /// Thread: minutes of inactivity before archiving
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u32>,

    /// Thread: only moderators can unarchive
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<bool>,

    /// Private thread: non-moderators can add members
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitable: Option<bool>,

    /// Forum thread: tag ids
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_tags: Option<&'a [Snowflake]>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Body of [`BotClient::edit_channel_permissions`]
#[derive(Debug, Clone, Copy, Serialize)]
pub struct EditChannelPermissions {
    #[serde(rename = "type")]
    pub kind: OverwriteType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<Permissions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<Permissions>,
}

#[derive(Serialize)]
struct FollowChannel {
    webhook_channel_id: Snowflake,
}

impl<T: Transport> BotClient<T> {
    /// [Get Channel](https://discord.com/developers/docs/resources/channel#get-channel)
    pub async fn get_channel(&self, channel_id: Snowflake) -> Result<Response<Channel>> {
        let url = self.rest.url(GET_CHANNEL, &[&channel_id]);
        self.rest.call(GET_CHANNEL, url, Body::NONE, None).await
    }

    /// [Modify Channel](https://discord.com/developers/docs/resources/channel#modify-channel)
    /// (and threads)
    pub async fn modify_channel(
        &self,
        channel_id: Snowflake,
        channel: &ModifyChannel<'_>,
    ) -> Result<Response<Channel>> {
        let url = self.rest.url(MODIFY_CHANNEL, &[&channel_id]);
        self.rest
            .call(MODIFY_CHANNEL, url, Body::Json(channel), channel.reason)
            .await
    }

    /// [Delete/Close Channel](https://discord.com/developers/docs/resources/channel#deleteclose-channel)
    pub async fn delete_channel(
        &self,
        channel_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<Response<Channel>> {
        let url = self.rest.url(DELETE_CHANNEL, &[&channel_id]);
        self.rest
            .call(DELETE_CHANNEL, url, Body::NONE, reason)
            .await
    }

    /// [Edit Channel Permissions](https://discord.com/developers/docs/resources/channel#edit-channel-permissions)
    /// for a role or member
    pub async fn edit_channel_permissions(
        &self,
        channel_id: Snowflake,
        overwrite_id: Snowflake,
        permissions: &EditChannelPermissions,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self
            .rest
            .url(EDIT_CHANNEL_PERMISSIONS, &[&channel_id, &overwrite_id]);
        self.rest
            .call_empty(
                EDIT_CHANNEL_PERMISSIONS,
                url,
                Body::Json(permissions),
                reason,
            )
            .await
    }

    /// [Delete Channel Permission](https://discord.com/developers/docs/resources/channel#delete-channel-permission)
    pub async fn delete_channel_permission(
        &self,
        channel_id: Snowflake,
        overwrite_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self
            .rest
            .url(DELETE_CHANNEL_PERMISSION, &[&channel_id, &overwrite_id]);
        self.rest
            .call_empty(DELETE_CHANNEL_PERMISSION, url, Body::NONE, reason)
            .await
    }

    /// [Trigger Typing Indicator](https://discord.com/developers/docs/resources/channel#trigger-typing-indicator)
    pub async fn trigger_typing(&self, channel_id: Snowflake) -> Result<()> {
        let url = self.rest.url(TRIGGER_TYPING, &[&channel_id]);
        self.rest
            .call_empty(TRIGGER_TYPING, url, Body::NONE, None)
            .await
    }

    /// [Follow Announcement Channel](https://discord.com/developers/docs/resources/channel#follow-announcement-channel):
    /// crosspost `channel_id`'s messages into `target_channel_id`
    pub async fn follow_announcement_channel(
        &self,
        channel_id: Snowflake,
        target_channel_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<Response<FollowedChannel>> {
        let url = self.rest.url(FOLLOW_ANNOUNCEMENT_CHANNEL, &[&channel_id]);
        let body = FollowChannel {
            webhook_channel_id: target_channel_id,
        };
        self.rest
            .call(FOLLOW_ANNOUNCEMENT_CHANNEL, url, Body::Json(&body), reason)
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
    fn routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        let (channel, other) = (Snowflake::from_u64(5), Snowflake::from_u64(6));
        let modify = ModifyChannel {
            name: Some("general"),
            topic: Some(None),
            reason: Some("tidy"),
            ..Default::default()
        };
        let permissions = EditChannelPermissions {
            kind: OverwriteType::Role,
            allow: Some(Permissions::SendMessages),
            deny: None,
        };
        block_on(c.get_channel(channel)).unwrap();
        block_on(c.modify_channel(channel, &modify)).unwrap();
        block_on(c.delete_channel(channel, None)).unwrap();
        block_on(c.edit_channel_permissions(channel, other, &permissions, None)).unwrap();
        block_on(c.delete_channel_permission(channel, other, None)).unwrap();
        block_on(c.trigger_typing(channel)).unwrap();
        block_on(c.follow_announcement_channel(channel, other, None)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/channels/5".into()),
                (Patch, "/channels/5".into()),
                (Delete, "/channels/5".into()),
                (Put, "/channels/5/permissions/6".into()),
                (Delete, "/channels/5/permissions/6".into()),
                (Post, "/channels/5/typing".into()),
                (Post, "/channels/5/followers".into()),
            ]
        );
        assert_eq!(t.body(1), json!({ "name": "general", "topic": null }));
        assert_eq!(t.request(1).header("x-audit-log-reason"), Some("tidy"));
        assert_eq!(t.body(3), json!({ "type": 0, "allow": "2048" }));
        assert_eq!(t.body(6), json!({ "webhook_channel_id": "6" }));
    }
}
