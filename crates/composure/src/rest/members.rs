//! [Guild Members](https://discord.com/developers/docs/resources/guild#list-guild-members): reading,
//! editing, roles, kicking

use serde::Serialize;

use crate::types::{Member, Snowflake};

use super::{
    Body, BotClient, Response, Result, Transport,
    route::{Query, routes},
};

routes! {
    GET_MEMBER = Get "/guilds/{guild_id}/members/{user_id}";
    LIST_MEMBERS = Get "/guilds/{guild_id}/members";
    SEARCH_MEMBERS = Get "/guilds/{guild_id}/members/search";
    ADD_MEMBER = Put "/guilds/{guild_id}/members/{user_id}";
    MODIFY_MEMBER = Patch "/guilds/{guild_id}/members/{user_id}";
    MODIFY_CURRENT_MEMBER = Patch "/guilds/{guild_id}/members/@me";
    ADD_MEMBER_ROLE = Put "/guilds/{guild_id}/members/{user_id}/roles/{role_id}";
    REMOVE_MEMBER_ROLE = Delete "/guilds/{guild_id}/members/{user_id}/roles/{role_id}";
    REMOVE_MEMBER = Delete "/guilds/{guild_id}/members/{user_id}";
}

/// Body of [`BotClient::modify_member`]. Absent fields are unchanged; `Some(None)` clears.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModifyMember<'a> {
    /// Server nickname
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<Option<&'a str>>,

    /// The full role list to set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<&'a [Snowflake]>,

    /// Server mute in voice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,

    /// Server deafen in voice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deaf: Option<bool>,

    /// Move to this voice channel; `Some(None)` disconnects
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Option<Snowflake>>,

    /// Timeout until (ISO 8601, at most 28 days ahead); `Some(None)` removes the timeout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication_disabled_until: Option<Option<&'a str>>,

    /// [Guild member flags](https://discord.com/developers/docs/resources/guild#guild-member-object-guild-member-flags)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Body of [`BotClient::modify_current_member`]: the bot's own server profile
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModifyCurrentMember<'a> {
    /// Server nickname
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<Option<&'a str>>,

    /// Data URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<Option<&'a str>>,

    /// Data URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<Option<&'a str>>,

    /// Server profile bio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bio: Option<Option<&'a str>>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Body of [`BotClient::add_member`]: adds a user who authorized the app with the `guilds.join`
/// scope
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct AddMember<'a> {
    /// The user's OAuth2 access token
    pub access_token: &'a str,

    /// Server nickname
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<&'a str>,

    /// Roles to give
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<&'a [Snowflake]>,

    /// Server mute in voice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,

    /// Server deafen in voice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deaf: Option<bool>,
}

impl<T: Transport> BotClient<T> {
    /// [Get Guild Member](https://discord.com/developers/docs/resources/guild#get-guild-member)
    pub async fn get_member(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Response<Member>> {
        let url = self.rest.url(GET_MEMBER, &[&guild_id, &user_id]);
        self.rest.call(GET_MEMBER, url, Body::NONE, None).await
    }

    /// [List Guild Members](https://discord.com/developers/docs/resources/guild#list-guild-members)
    /// by user id, paged by `after` (1-1000, default 1); needs the members privileged intent
    pub async fn list_members(
        &self,
        guild_id: Snowflake,
        after: Option<Snowflake>,
        limit: Option<u16>,
    ) -> Result<Response<Vec<Member>>> {
        let mut url = self.rest.url(LIST_MEMBERS, &[&guild_id]);
        Query::new(&mut url)
            .push_opt("limit", limit)
            .push_opt("after", after);
        self.rest.call(LIST_MEMBERS, url, Body::NONE, None).await
    }

    /// [Search Guild Members](https://discord.com/developers/docs/resources/guild#search-guild-members)
    /// whose username or nickname starts with `query` (1-1000, default 1)
    pub async fn search_members(
        &self,
        guild_id: Snowflake,
        query: &str,
        limit: Option<u16>,
    ) -> Result<Response<Vec<Member>>> {
        let mut url = self.rest.url(SEARCH_MEMBERS, &[&guild_id]);
        Query::new(&mut url)
            .push("query", query)
            .push_opt("limit", limit);
        self.rest.call(SEARCH_MEMBERS, url, Body::NONE, None).await
    }

    /// [Add Guild Member](https://discord.com/developers/docs/resources/guild#add-guild-member).
    /// Status 201 with the member, or 204 and no body if they already were one.
    pub async fn add_member(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
        member: &AddMember<'_>,
    ) -> Result<Response<Member>> {
        let url = self.rest.url(ADD_MEMBER, &[&guild_id, &user_id]);
        self.rest
            .call(ADD_MEMBER, url, Body::Json(member), None)
            .await
    }

    /// [Modify Guild Member](https://discord.com/developers/docs/resources/guild#modify-guild-member):
    /// nickname, roles, voice state, timeout
    pub async fn modify_member(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
        member: &ModifyMember<'_>,
    ) -> Result<Response<Member>> {
        let url = self.rest.url(MODIFY_MEMBER, &[&guild_id, &user_id]);
        self.rest
            .call(MODIFY_MEMBER, url, Body::Json(member), member.reason)
            .await
    }

    /// [Modify Current Member](https://discord.com/developers/docs/resources/guild#modify-current-member)
    pub async fn modify_current_member(
        &self,
        guild_id: Snowflake,
        member: &ModifyCurrentMember<'_>,
    ) -> Result<Response<Member>> {
        let url = self.rest.url(MODIFY_CURRENT_MEMBER, &[&guild_id]);
        self.rest
            .call(
                MODIFY_CURRENT_MEMBER,
                url,
                Body::Json(member),
                member.reason,
            )
            .await
    }

    /// [Add Guild Member Role](https://discord.com/developers/docs/resources/guild#add-guild-member-role)
    pub async fn add_member_role(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
        role_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self
            .rest
            .url(ADD_MEMBER_ROLE, &[&guild_id, &user_id, &role_id]);
        self.rest
            .call_empty(ADD_MEMBER_ROLE, url, Body::NONE, reason)
            .await
    }

    /// [Remove Guild Member Role](https://discord.com/developers/docs/resources/guild#remove-guild-member-role)
    pub async fn remove_member_role(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
        role_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self
            .rest
            .url(REMOVE_MEMBER_ROLE, &[&guild_id, &user_id, &role_id]);
        self.rest
            .call_empty(REMOVE_MEMBER_ROLE, url, Body::NONE, reason)
            .await
    }

    /// [Remove Guild Member](https://discord.com/developers/docs/resources/guild#remove-guild-member):
    /// kick
    pub async fn remove_member(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(REMOVE_MEMBER, &[&guild_id, &user_id]);
        self.rest
            .call_empty(REMOVE_MEMBER, url, Body::NONE, reason)
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
        let (g, u, r) = (
            Snowflake::from_u64(5),
            Snowflake::from_u64(6),
            Snowflake::from_u64(7),
        );
        let timeout = ModifyMember {
            communication_disabled_until: Some(Some("2030-01-01T00:00:00Z")),
            nick: Some(None),
            reason: Some("spam"),
            ..Default::default()
        };
        block_on(c.get_member(g, u)).unwrap();
        block_on(c.list_members(g, Some(u), Some(1000))).unwrap();
        block_on(c.search_members(g, "ana b", None)).unwrap();
        block_on(c.add_member(
            g,
            u,
            &AddMember {
                access_token: "tok",
                ..Default::default()
            },
        ))
        .unwrap();
        block_on(c.modify_member(g, u, &timeout)).unwrap();
        block_on(c.modify_current_member(
            g,
            &ModifyCurrentMember {
                nick: Some(Some("Bot")),
                ..Default::default()
            },
        ))
        .unwrap();
        block_on(c.add_member_role(g, u, r, None)).unwrap();
        block_on(c.remove_member_role(g, u, r, None)).unwrap();
        block_on(c.remove_member(g, u, Some("bye"))).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/guilds/5/members/6".into()),
                (Get, "/guilds/5/members?limit=1000&after=6".into()),
                (Get, "/guilds/5/members/search?query=ana%20b".into()),
                (Put, "/guilds/5/members/6".into()),
                (Patch, "/guilds/5/members/6".into()),
                (Patch, "/guilds/5/members/@me".into()),
                (Put, "/guilds/5/members/6/roles/7".into()),
                (Delete, "/guilds/5/members/6/roles/7".into()),
                (Delete, "/guilds/5/members/6".into()),
            ]
        );
        assert_eq!(t.body(3), json!({ "access_token": "tok" }));
        assert_eq!(
            t.body(4),
            json!({ "nick": null, "communication_disabled_until": "2030-01-01T00:00:00Z" })
        );
        assert_eq!(t.request(8).header("x-audit-log-reason"), Some("bye"));
    }
}
