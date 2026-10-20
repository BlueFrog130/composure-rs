//! [Bans](https://discord.com/developers/docs/resources/guild#get-guild-bans)

use serde::Serialize;

use crate::types::{Ban, BulkBan, Snowflake};

use super::{
    Body, BotClient, Response, Result, Transport,
    route::{Query, routes},
};

routes! {
    GET_BANS = Get "/guilds/{guild_id}/bans";
    GET_BAN = Get "/guilds/{guild_id}/bans/{user_id}";
    CREATE_BAN = Put "/guilds/{guild_id}/bans/{user_id}";
    REMOVE_BAN = Delete "/guilds/{guild_id}/bans/{user_id}";
    BULK_BAN = Post "/guilds/{guild_id}/bulk-ban";
}

/// Paging of [`BotClient::get_bans`], by user id
#[derive(Debug, Clone, Copy, Default)]
pub struct GetBans {
    /// Bans of users before this id
    pub before: Option<Snowflake>,
    /// Bans of users after this id
    pub after: Option<Snowflake>,
    /// 1-1000, default 1000
    pub limit: Option<u16>,
}

#[derive(Serialize)]
struct CreateBan {
    #[serde(skip_serializing_if = "Option::is_none")]
    delete_message_seconds: Option<u32>,
}

#[derive(Serialize)]
struct BulkBanBody<'a> {
    user_ids: &'a [Snowflake],
    #[serde(skip_serializing_if = "Option::is_none")]
    delete_message_seconds: Option<u32>,
}

impl<T: Transport> BotClient<T> {
    /// [Get Guild Bans](https://discord.com/developers/docs/resources/guild#get-guild-bans)
    pub async fn get_bans(
        &self,
        guild_id: Snowflake,
        query: &GetBans,
    ) -> Result<Response<Vec<Ban>>> {
        let mut url = self.rest.url(GET_BANS, &[&guild_id]);
        Query::new(&mut url)
            .push_opt("limit", query.limit)
            .push_opt("before", query.before)
            .push_opt("after", query.after);
        self.rest.call(GET_BANS, url, Body::NONE, None).await
    }

    /// [Get Guild Ban](https://discord.com/developers/docs/resources/guild#get-guild-ban)
    pub async fn get_ban(&self, guild_id: Snowflake, user_id: Snowflake) -> Result<Response<Ban>> {
        let url = self.rest.url(GET_BAN, &[&guild_id, &user_id]);
        self.rest.call(GET_BAN, url, Body::NONE, None).await
    }

    /// [Create Guild Ban](https://discord.com/developers/docs/resources/guild#create-guild-ban),
    /// deleting the user's messages from the last `delete_message_seconds` (max 7 days)
    pub async fn create_ban(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
        delete_message_seconds: Option<u32>,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(CREATE_BAN, &[&guild_id, &user_id]);
        let body = CreateBan {
            delete_message_seconds,
        };
        self.rest
            .call_empty(CREATE_BAN, url, Body::Json(&body), reason)
            .await
    }

    /// [Remove Guild Ban](https://discord.com/developers/docs/resources/guild#remove-guild-ban)
    pub async fn remove_ban(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(REMOVE_BAN, &[&guild_id, &user_id]);
        self.rest
            .call_empty(REMOVE_BAN, url, Body::NONE, reason)
            .await
    }

    /// [Bulk Guild Ban](https://discord.com/developers/docs/resources/guild#bulk-guild-ban): up
    /// to 200 users
    pub async fn bulk_ban(
        &self,
        guild_id: Snowflake,
        user_ids: &[Snowflake],
        delete_message_seconds: Option<u32>,
        reason: Option<&str>,
    ) -> Result<Response<BulkBan>> {
        let url = self.rest.url(BULK_BAN, &[&guild_id]);
        let body = BulkBanBody {
            user_ids,
            delete_message_seconds,
        };
        self.rest
            .call(BULK_BAN, url, Body::Json(&body), reason)
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
        let (g, u) = (Snowflake::from_u64(5), Snowflake::from_u64(6));
        block_on(c.get_bans(
            g,
            &GetBans {
                after: Some(u),
                ..Default::default()
            },
        ))
        .unwrap();
        block_on(c.get_ban(g, u)).unwrap();
        block_on(c.create_ban(g, u, Some(3600), Some("spam"))).unwrap();
        block_on(c.remove_ban(g, u, None)).unwrap();
        block_on(c.bulk_ban(g, &[u], None, None)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/guilds/5/bans?after=6".into()),
                (Get, "/guilds/5/bans/6".into()),
                (Put, "/guilds/5/bans/6".into()),
                (Delete, "/guilds/5/bans/6".into()),
                (Post, "/guilds/5/bulk-ban".into()),
            ]
        );
        assert_eq!(t.body(2), json!({ "delete_message_seconds": 3600 }));
        assert_eq!(t.request(2).header("x-audit-log-reason"), Some("spam"));
        assert_eq!(t.body(4), json!({ "user_ids": ["6"] }));
    }
}
