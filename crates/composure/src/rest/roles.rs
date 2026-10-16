//! [Roles](https://discord.com/developers/docs/resources/guild#get-guild-roles)

use std::collections::HashMap;

use serde::Serialize;

use crate::types::{Permissions, Role, Snowflake};

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    GET_ROLES = Get "/guilds/{guild_id}/roles";
    GET_ROLE = Get "/guilds/{guild_id}/roles/{role_id}";
    CREATE_ROLE = Post "/guilds/{guild_id}/roles";
    MODIFY_ROLE = Patch "/guilds/{guild_id}/roles/{role_id}";
    MODIFY_ROLE_POSITIONS = Patch "/guilds/{guild_id}/roles";
    DELETE_ROLE = Delete "/guilds/{guild_id}/roles/{role_id}";
    GET_ROLE_MEMBER_COUNTS = Get "/guilds/{guild_id}/roles/member-counts";
}

/// Body of [`BotClient::create_role`] and [`BotClient::modify_role`]. Absent fields take
/// Discord's default (create) or are unchanged (modify); `Some(None)` clears.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct RoleFields<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,

    /// RGB
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,

    /// Shown separately in the member list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hoist: Option<bool>,

    /// Data URI (needs the ROLE_ICONS feature)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<Option<&'a str>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentionable: Option<bool>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// One entry of [`BotClient::modify_role_positions`]
#[derive(Debug, Clone, Copy, Serialize)]
pub struct RolePosition {
    pub id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<i32>,
}

impl<T: Transport> BotClient<T> {
    /// [Get Guild Roles](https://discord.com/developers/docs/resources/guild#get-guild-roles)
    pub async fn get_roles(&self, guild_id: Snowflake) -> Result<Response<Vec<Role>>> {
        let url = self.rest.url(GET_ROLES, &[&guild_id]);
        self.rest.call(GET_ROLES, url, Body::NONE, None).await
    }

    /// [Get Guild Role](https://discord.com/developers/docs/resources/guild#get-guild-role)
    pub async fn get_role(
        &self,
        guild_id: Snowflake,
        role_id: Snowflake,
    ) -> Result<Response<Role>> {
        let url = self.rest.url(GET_ROLE, &[&guild_id, &role_id]);
        self.rest.call(GET_ROLE, url, Body::NONE, None).await
    }

    /// [Create Guild Role](https://discord.com/developers/docs/resources/guild#create-guild-role)
    pub async fn create_role(
        &self,
        guild_id: Snowflake,
        role: &RoleFields<'_>,
    ) -> Result<Response<Role>> {
        let url = self.rest.url(CREATE_ROLE, &[&guild_id]);
        self.rest
            .call(CREATE_ROLE, url, Body::Json(role), role.reason)
            .await
    }

    /// [Modify Guild Role](https://discord.com/developers/docs/resources/guild#modify-guild-role)
    pub async fn modify_role(
        &self,
        guild_id: Snowflake,
        role_id: Snowflake,
        role: &RoleFields<'_>,
    ) -> Result<Response<Role>> {
        let url = self.rest.url(MODIFY_ROLE, &[&guild_id, &role_id]);
        self.rest
            .call(MODIFY_ROLE, url, Body::Json(role), role.reason)
            .await
    }

    /// [Modify Guild Role Positions](https://discord.com/developers/docs/resources/guild#modify-guild-role-positions)
    pub async fn modify_role_positions(
        &self,
        guild_id: Snowflake,
        positions: &[RolePosition],
        reason: Option<&str>,
    ) -> Result<Response<Vec<Role>>> {
        let url = self.rest.url(MODIFY_ROLE_POSITIONS, &[&guild_id]);
        self.rest
            .call(MODIFY_ROLE_POSITIONS, url, Body::Json(&positions), reason)
            .await
    }

    /// [Delete Guild Role](https://discord.com/developers/docs/resources/guild#delete-guild-role)
    pub async fn delete_role(
        &self,
        guild_id: Snowflake,
        role_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(DELETE_ROLE, &[&guild_id, &role_id]);
        self.rest
            .call_empty(DELETE_ROLE, url, Body::NONE, reason)
            .await
    }

    /// [Get Guild Role Member Counts](https://discord.com/developers/docs/resources/guild#get-guild-role-member-counts):
    /// members per role id (`@everyone` excluded)
    pub async fn get_role_member_counts(
        &self,
        guild_id: Snowflake,
    ) -> Result<Response<HashMap<Snowflake, u32>>> {
        let url = self.rest.url(GET_ROLE_MEMBER_COUNTS, &[&guild_id]);
        self.rest
            .call(GET_ROLE_MEMBER_COUNTS, url, Body::NONE, None)
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
        let c = BotClient::new(
            MockTransport::default().respond(200, "[]"),
            "t",
            Snowflake::from_u64(1),
        );
        let (g, r) = (Snowflake::from_u64(5), Snowflake::from_u64(6));
        let fields = RoleFields {
            name: Some("Mods"),
            permissions: Some(Permissions::KickMembers | Permissions::BanMembers),
            color: Some(0xff0000),
            icon: Some(None),
            reason: Some("staffing"),
            ..Default::default()
        };
        block_on(c.get_roles(g)).unwrap();
        block_on(c.get_role(g, r)).unwrap();
        block_on(c.create_role(g, &fields)).unwrap();
        block_on(c.modify_role(g, r, &fields)).unwrap();
        block_on(c.modify_role_positions(
            g,
            &[RolePosition {
                id: r,
                position: Some(3),
            }],
            None,
        ))
        .unwrap();
        block_on(c.delete_role(g, r, None)).unwrap();
        let t = c.transport();
        t.responses
            .borrow_mut()
            .push_back(super::super::RawResponse {
                status: 200,
                headers: Vec::new(),
                body: br#"{"6":12,"7":0}"#.to_vec(),
            });
        let counts = block_on(c.get_role_member_counts(g))
            .unwrap()
            .model()
            .unwrap();
        assert_eq!(counts[&r], 12);
        assert_eq!(
            t.sent(),
            [
                (Get, "/guilds/5/roles".into()),
                (Get, "/guilds/5/roles/6".into()),
                (Post, "/guilds/5/roles".into()),
                (Patch, "/guilds/5/roles/6".into()),
                (Patch, "/guilds/5/roles".into()),
                (Delete, "/guilds/5/roles/6".into()),
                (Get, "/guilds/5/roles/member-counts".into()),
            ]
        );
        assert_eq!(
            t.body(2),
            json!({ "name": "Mods", "permissions": "6", "color": 16711680, "icon": null })
        );
        assert_eq!(t.body(4), json!([{ "id": "6", "position": 3 }]));
    }
}
