//! [Guilds](https://discord.com/developers/docs/resources/guild#endpoints): settings, preview,
//! channels

use serde::Serialize;

use crate::types::{
    Channel, ChannelType, DefaultMessageNotificationLevel, DefaultReaction,
    ExplicitContentFilterLevel, ForumLayoutType, Guild, GuildPreview, Overwrite, PartialGuild,
    Snowflake, SortOrderType, VerificationLevel, VideoQualityMode,
};

use super::{
    Body, BotClient, Response, Result, Transport,
    route::{Query, routes},
};

routes! {
    GET_GUILD = Get "/guilds/{guild_id}";
    MODIFY_GUILD = Patch "/guilds/{guild_id}";
    GET_GUILD_PREVIEW = Get "/guilds/{guild_id}/preview";
    GET_GUILD_CHANNELS = Get "/guilds/{guild_id}/channels";
    CREATE_GUILD_CHANNEL = Post "/guilds/{guild_id}/channels";
    MODIFY_GUILD_CHANNEL_POSITIONS = Patch "/guilds/{guild_id}/channels";
    GET_CURRENT_USER_GUILDS = Get "/users/@me/guilds";
    LEAVE_GUILD = Delete "/users/@me/guilds/{guild_id}";
}

/// Body of [`BotClient::modify_guild`]. Absent fields are unchanged; `Some(None)` clears a
/// nullable one. Images are data URIs (`data:image/png;base64,...`).
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModifyGuild<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_level: Option<Option<VerificationLevel>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_message_notifications: Option<Option<DefaultMessageNotificationLevel>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_content_filter: Option<Option<ExplicitContentFilterLevel>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub afk_channel_id: Option<Option<Snowflake>>,

    /// Seconds: 60, 300, 900, 1800, or 3600
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afk_timeout: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub splash: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_splash: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_channel_id: Option<Option<Snowflake>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_channel_flags: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_channel_id: Option<Option<Snowflake>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_updates_channel_id: Option<Option<Snowflake>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_alerts_channel_id: Option<Option<Snowflake>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_locale: Option<Option<&'a str>>,

    /// The full feature list to set (only some are mutable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<&'a [&'a str]>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_progress_bar_enabled: Option<bool>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Body of [`BotClient::create_guild_channel`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CreateGuildChannel<'a> {
    pub name: &'a str,

    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<ChannelType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_overwrites: Option<&'a [Overwrite]>,

    /// Category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Snowflake>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_region: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<VideoQualityMode>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reaction_emoji: Option<&'a DefaultReaction>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_sort_order: Option<SortOrderType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_forum_layout: Option<ForumLayoutType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u32>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// One entry of [`BotClient::modify_guild_channel_positions`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ChannelPosition {
    pub id: Snowflake,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<Option<i32>>,

    /// Sync permissions with the new parent category
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock_permissions: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Option<Snowflake>>,
}

/// Paging of [`BotClient::get_current_user_guilds`]
#[derive(Debug, Clone, Copy, Default)]
pub struct GetGuilds {
    pub before: Option<Snowflake>,
    pub after: Option<Snowflake>,
    /// 1-200, default 200
    pub limit: Option<u8>,
    pub with_counts: bool,
}

impl<T: Transport> BotClient<T> {
    /// [Get Guild](https://discord.com/developers/docs/resources/guild#get-guild); `with_counts`
    /// fills the approximate member and presence counts
    pub async fn get_guild(
        &self,
        guild_id: Snowflake,
        with_counts: bool,
    ) -> Result<Response<Guild>> {
        let mut url = self.rest.url(GET_GUILD, &[&guild_id]);
        if with_counts {
            Query::new(&mut url).push("with_counts", true);
        }
        self.rest.call(GET_GUILD, url, Body::NONE, None).await
    }

    /// [Modify Guild](https://discord.com/developers/docs/resources/guild#modify-guild)
    pub async fn modify_guild(
        &self,
        guild_id: Snowflake,
        guild: &ModifyGuild<'_>,
    ) -> Result<Response<Guild>> {
        let url = self.rest.url(MODIFY_GUILD, &[&guild_id]);
        self.rest
            .call(MODIFY_GUILD, url, Body::Json(guild), guild.reason)
            .await
    }

    /// [Get Guild Preview](https://discord.com/developers/docs/resources/guild#get-guild-preview)
    pub async fn get_guild_preview(&self, guild_id: Snowflake) -> Result<Response<GuildPreview>> {
        let url = self.rest.url(GET_GUILD_PREVIEW, &[&guild_id]);
        self.rest
            .call(GET_GUILD_PREVIEW, url, Body::NONE, None)
            .await
    }

    /// [Get Guild Channels](https://discord.com/developers/docs/resources/guild#get-guild-channels)
    /// (no threads)
    pub async fn get_guild_channels(&self, guild_id: Snowflake) -> Result<Response<Vec<Channel>>> {
        let url = self.rest.url(GET_GUILD_CHANNELS, &[&guild_id]);
        self.rest
            .call(GET_GUILD_CHANNELS, url, Body::NONE, None)
            .await
    }

    /// [Create Guild Channel](https://discord.com/developers/docs/resources/guild#create-guild-channel)
    pub async fn create_guild_channel(
        &self,
        guild_id: Snowflake,
        channel: &CreateGuildChannel<'_>,
    ) -> Result<Response<Channel>> {
        let url = self.rest.url(CREATE_GUILD_CHANNEL, &[&guild_id]);
        self.rest
            .call(
                CREATE_GUILD_CHANNEL,
                url,
                Body::Json(channel),
                channel.reason,
            )
            .await
    }

    /// [Modify Guild Channel Positions](https://discord.com/developers/docs/resources/guild#modify-guild-channel-positions)
    pub async fn modify_guild_channel_positions(
        &self,
        guild_id: Snowflake,
        positions: &[ChannelPosition],
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(MODIFY_GUILD_CHANNEL_POSITIONS, &[&guild_id]);
        self.rest
            .call_empty(
                MODIFY_GUILD_CHANNEL_POSITIONS,
                url,
                Body::Json(&positions),
                reason,
            )
            .await
    }

    /// [Get Current User Guilds](https://discord.com/developers/docs/resources/user#get-current-user-guilds):
    /// the guilds the bot is in
    pub async fn get_current_user_guilds(
        &self,
        query: &GetGuilds,
    ) -> Result<Response<Vec<PartialGuild>>> {
        let mut url = self.rest.url(GET_CURRENT_USER_GUILDS, &[]);
        Query::new(&mut url)
            .push_opt("before", query.before)
            .push_opt("after", query.after)
            .push_opt("limit", query.limit)
            .push_opt("with_counts", query.with_counts.then_some(true));
        self.rest
            .call(GET_CURRENT_USER_GUILDS, url, Body::NONE, None)
            .await
    }

    /// [Leave Guild](https://discord.com/developers/docs/resources/user#leave-guild)
    pub async fn leave_guild(&self, guild_id: Snowflake) -> Result<()> {
        let url = self.rest.url(LEAVE_GUILD, &[&guild_id]);
        self.rest
            .call_empty(LEAVE_GUILD, url, Body::NONE, None)
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
        let g = Snowflake::from_u64(5);
        let modify = ModifyGuild {
            name: Some("Home"),
            afk_channel_id: Some(None),
            verification_level: Some(Some(VerificationLevel::High)),
            ..Default::default()
        };
        let channel = CreateGuildChannel {
            name: "voice",
            kind: Some(ChannelType::GuildVoice),
            user_limit: Some(5),
            reason: Some("events"),
            ..Default::default()
        };
        let positions = [ChannelPosition {
            id: Snowflake::from_u64(6),
            position: Some(Some(2)),
            ..Default::default()
        }];
        block_on(c.get_guild(g, true)).unwrap();
        block_on(c.modify_guild(g, &modify)).unwrap();
        block_on(c.get_guild_preview(g)).unwrap();
        block_on(c.get_guild_channels(g)).unwrap();
        block_on(c.create_guild_channel(g, &channel)).unwrap();
        block_on(c.modify_guild_channel_positions(g, &positions, None)).unwrap();
        block_on(c.get_current_user_guilds(&GetGuilds {
            limit: Some(10),
            ..Default::default()
        }))
        .unwrap();
        block_on(c.leave_guild(g)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/guilds/5?with_counts=true".into()),
                (Patch, "/guilds/5".into()),
                (Get, "/guilds/5/preview".into()),
                (Get, "/guilds/5/channels".into()),
                (Post, "/guilds/5/channels".into()),
                (Patch, "/guilds/5/channels".into()),
                (Get, "/users/@me/guilds?limit=10".into()),
                (Delete, "/users/@me/guilds/5".into()),
            ]
        );
        assert_eq!(
            t.body(1),
            json!({ "name": "Home", "verification_level": 3, "afk_channel_id": null })
        );
        assert_eq!(
            t.body(4),
            json!({ "name": "voice", "type": 2, "user_limit": 5 })
        );
        assert_eq!(t.request(4).header("x-audit-log-reason"), Some("events"));
        assert_eq!(t.body(5), json!([{ "id": "6", "position": 2 }]));
    }
}
