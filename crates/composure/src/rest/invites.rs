//! [Invites](https://discord.com/developers/docs/resources/invite)

use serde::Serialize;

use crate::types::{Invite, InviteTargetType, Snowflake};

use super::{
    Body, BotClient, Response, Result, Transport,
    route::{PathSegment, Query, routes},
};

routes! {
    GET_CHANNEL_INVITES = Get "/channels/{channel_id}/invites";
    CREATE_CHANNEL_INVITE = Post "/channels/{channel_id}/invites";
    GET_GUILD_INVITES = Get "/guilds/{guild_id}/invites";
    GET_INVITE = Get "/invites/{code}";
    DELETE_INVITE = Delete "/invites/{code}";
}

/// Body of [`BotClient::create_channel_invite`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CreateInvite<'a> {
    /// Lifetime in seconds (0 forever, default 86400, max 604800)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<u32>,

    /// Use limit (0 unlimited, max 100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u32>,

    /// Members are kicked when they disconnect unless given a role
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,

    /// Don't reuse a similar existing invite
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_type: Option<InviteTargetType>,

    /// Stream invites: whose stream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_user_id: Option<Snowflake>,

    /// Embedded application invites: which activity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_application_id: Option<Snowflake>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

impl<T: Transport> BotClient<T> {
    /// [Get Channel Invites](https://discord.com/developers/docs/resources/channel#get-channel-invites),
    /// with metadata
    pub async fn get_channel_invites(
        &self,
        channel_id: Snowflake,
    ) -> Result<Response<Vec<Invite>>> {
        let url = self.rest.url(GET_CHANNEL_INVITES, &[&channel_id]);
        self.rest
            .call(GET_CHANNEL_INVITES, url, Body::NONE, None)
            .await
    }

    /// [Create Channel Invite](https://discord.com/developers/docs/resources/channel#create-channel-invite)
    pub async fn create_channel_invite(
        &self,
        channel_id: Snowflake,
        invite: &CreateInvite<'_>,
    ) -> Result<Response<Invite>> {
        let url = self.rest.url(CREATE_CHANNEL_INVITE, &[&channel_id]);
        self.rest
            .call(
                CREATE_CHANNEL_INVITE,
                url,
                Body::Json(invite),
                invite.reason,
            )
            .await
    }

    /// [Get Guild Invites](https://discord.com/developers/docs/resources/guild#get-guild-invites),
    /// with metadata
    pub async fn get_guild_invites(&self, guild_id: Snowflake) -> Result<Response<Vec<Invite>>> {
        let url = self.rest.url(GET_GUILD_INVITES, &[&guild_id]);
        self.rest
            .call(GET_GUILD_INVITES, url, Body::NONE, None)
            .await
    }

    /// [Get Invite](https://discord.com/developers/docs/resources/invite#get-invite); `with_counts`
    /// fills the approximate member counts
    pub async fn get_invite(&self, code: &str, with_counts: bool) -> Result<Response<Invite>> {
        let mut url = self.rest.url(GET_INVITE, &[&PathSegment(code)]);
        Query::new(&mut url).push_opt("with_counts", with_counts.then_some(true));
        self.rest.call(GET_INVITE, url, Body::NONE, None).await
    }

    /// [Delete Invite](https://discord.com/developers/docs/resources/invite#delete-invite)
    pub async fn delete_invite(
        &self,
        code: &str,
        reason: Option<&str>,
    ) -> Result<Response<Invite>> {
        let url = self.rest.url(DELETE_INVITE, &[&PathSegment(code)]);
        self.rest.call(DELETE_INVITE, url, Body::NONE, reason).await
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
        let id = Snowflake::from_u64(5);
        let invite = CreateInvite {
            max_age: Some(0),
            unique: Some(true),
            ..Default::default()
        };
        block_on(c.get_channel_invites(id)).unwrap();
        block_on(c.create_channel_invite(id, &invite)).unwrap();
        block_on(c.get_guild_invites(id)).unwrap();
        block_on(c.get_invite("abc", true)).unwrap();
        block_on(c.delete_invite("abc", Some("leaked"))).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/channels/5/invites".into()),
                (Post, "/channels/5/invites".into()),
                (Get, "/guilds/5/invites".into()),
                (Get, "/invites/abc?with_counts=true".into()),
                (Delete, "/invites/abc".into()),
            ]
        );
        assert_eq!(t.body(1), json!({ "max_age": 0, "unique": true }));
    }

    #[test]
    fn invites_deserialize() {
        let invite: Invite = serde_json::from_value(json!({
            "type": 0, "code": "abc", "expires_at": null, "flags": 0,
            "guild": { "id": "1", "name": "Home", "icon": null, "features": [] },
            "channel": { "id": "2", "type": 0, "name": "general" },
            "uses": 3, "max_uses": 0, "max_age": 0, "temporary": false,
            "created_at": "2024-01-01T00:00:00+00:00",
        }))
        .unwrap();
        assert_eq!(invite.guild.unwrap().name, "Home");
        assert_eq!(invite.uses, Some(3));
    }
}
