//! [Emojis](https://discord.com/developers/docs/resources/emoji#endpoints): guild emojis and
//! application emojis (usable by the app everywhere)

use serde::{Deserialize, Serialize};

use crate::types::{Emoji, Snowflake};

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    LIST_GUILD_EMOJIS = Get "/guilds/{guild_id}/emojis";
    GET_GUILD_EMOJI = Get "/guilds/{guild_id}/emojis/{emoji_id}";
    CREATE_GUILD_EMOJI = Post "/guilds/{guild_id}/emojis";
    MODIFY_GUILD_EMOJI = Patch "/guilds/{guild_id}/emojis/{emoji_id}";
    DELETE_GUILD_EMOJI = Delete "/guilds/{guild_id}/emojis/{emoji_id}";
    LIST_APPLICATION_EMOJIS = Get "/applications/{application_id}/emojis";
    GET_APPLICATION_EMOJI = Get "/applications/{application_id}/emojis/{emoji_id}";
    CREATE_APPLICATION_EMOJI = Post "/applications/{application_id}/emojis";
    MODIFY_APPLICATION_EMOJI = Patch "/applications/{application_id}/emojis/{emoji_id}";
    DELETE_APPLICATION_EMOJI = Delete "/applications/{application_id}/emojis/{emoji_id}";
}

/// Body of [`BotClient::create_guild_emoji`] and [`BotClient::create_application_emoji`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CreateEmoji<'a> {
    /// 2-32 characters
    pub name: &'a str,

    /// Data URI of a 128x128 image (max 256 KiB)
    pub image: &'a str,

    /// Guild emojis only: roles allowed to use it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<&'a [Snowflake]>,

    /// `X-Audit-Log-Reason` (guild emojis)
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Body of [`BotClient::modify_guild_emoji`] and [`BotClient::modify_application_emoji`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModifyEmoji<'a> {
    /// 2-32 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,

    /// Guild emojis only; `Some(None)` allows everyone
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roles: Option<Option<&'a [Snowflake]>>,

    /// `X-Audit-Log-Reason` (guild emojis)
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Application emojis come wrapped
#[derive(Debug, Clone, Deserialize)]
pub struct ApplicationEmojis {
    /// The emojis
    pub items: Vec<Emoji>,
}

impl<T: Transport> BotClient<T> {
    /// [List Guild Emojis](https://discord.com/developers/docs/resources/emoji#list-guild-emojis)
    pub async fn list_guild_emojis(&self, guild_id: Snowflake) -> Result<Response<Vec<Emoji>>> {
        let url = self.rest.url(LIST_GUILD_EMOJIS, &[&guild_id]);
        self.rest
            .call(LIST_GUILD_EMOJIS, url, Body::NONE, None)
            .await
    }

    /// [Get Guild Emoji](https://discord.com/developers/docs/resources/emoji#get-guild-emoji)
    pub async fn get_guild_emoji(
        &self,
        guild_id: Snowflake,
        emoji_id: Snowflake,
    ) -> Result<Response<Emoji>> {
        let url = self.rest.url(GET_GUILD_EMOJI, &[&guild_id, &emoji_id]);
        self.rest.call(GET_GUILD_EMOJI, url, Body::NONE, None).await
    }

    /// [Create Guild Emoji](https://discord.com/developers/docs/resources/emoji#create-guild-emoji)
    pub async fn create_guild_emoji(
        &self,
        guild_id: Snowflake,
        emoji: &CreateEmoji<'_>,
    ) -> Result<Response<Emoji>> {
        let url = self.rest.url(CREATE_GUILD_EMOJI, &[&guild_id]);
        self.rest
            .call(CREATE_GUILD_EMOJI, url, Body::Json(emoji), emoji.reason)
            .await
    }

    /// [Modify Guild Emoji](https://discord.com/developers/docs/resources/emoji#modify-guild-emoji)
    pub async fn modify_guild_emoji(
        &self,
        guild_id: Snowflake,
        emoji_id: Snowflake,
        emoji: &ModifyEmoji<'_>,
    ) -> Result<Response<Emoji>> {
        let url = self.rest.url(MODIFY_GUILD_EMOJI, &[&guild_id, &emoji_id]);
        self.rest
            .call(MODIFY_GUILD_EMOJI, url, Body::Json(emoji), emoji.reason)
            .await
    }

    /// [Delete Guild Emoji](https://discord.com/developers/docs/resources/emoji#delete-guild-emoji)
    pub async fn delete_guild_emoji(
        &self,
        guild_id: Snowflake,
        emoji_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(DELETE_GUILD_EMOJI, &[&guild_id, &emoji_id]);
        self.rest
            .call_empty(DELETE_GUILD_EMOJI, url, Body::NONE, reason)
            .await
    }

    /// [List Application Emojis](https://discord.com/developers/docs/resources/emoji#list-application-emojis)
    pub async fn list_application_emojis(&self) -> Result<Response<ApplicationEmojis>> {
        let url = self
            .rest
            .url(LIST_APPLICATION_EMOJIS, &[&self.application_id]);
        self.rest
            .call(LIST_APPLICATION_EMOJIS, url, Body::NONE, None)
            .await
    }

    /// [Get Application Emoji](https://discord.com/developers/docs/resources/emoji#get-application-emoji)
    pub async fn get_application_emoji(&self, emoji_id: Snowflake) -> Result<Response<Emoji>> {
        let url = self
            .rest
            .url(GET_APPLICATION_EMOJI, &[&self.application_id, &emoji_id]);
        self.rest
            .call(GET_APPLICATION_EMOJI, url, Body::NONE, None)
            .await
    }

    /// [Create Application Emoji](https://discord.com/developers/docs/resources/emoji#create-application-emoji)
    pub async fn create_application_emoji(
        &self,
        emoji: &CreateEmoji<'_>,
    ) -> Result<Response<Emoji>> {
        let url = self
            .rest
            .url(CREATE_APPLICATION_EMOJI, &[&self.application_id]);
        self.rest
            .call(CREATE_APPLICATION_EMOJI, url, Body::Json(emoji), None)
            .await
    }

    /// [Modify Application Emoji](https://discord.com/developers/docs/resources/emoji#modify-application-emoji)
    pub async fn modify_application_emoji(
        &self,
        emoji_id: Snowflake,
        emoji: &ModifyEmoji<'_>,
    ) -> Result<Response<Emoji>> {
        let url = self
            .rest
            .url(MODIFY_APPLICATION_EMOJI, &[&self.application_id, &emoji_id]);
        self.rest
            .call(MODIFY_APPLICATION_EMOJI, url, Body::Json(emoji), None)
            .await
    }

    /// [Delete Application Emoji](https://discord.com/developers/docs/resources/emoji#delete-application-emoji)
    pub async fn delete_application_emoji(&self, emoji_id: Snowflake) -> Result<()> {
        let url = self
            .rest
            .url(DELETE_APPLICATION_EMOJI, &[&self.application_id, &emoji_id]);
        self.rest
            .call_empty(DELETE_APPLICATION_EMOJI, url, Body::NONE, None)
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
        let (g, e, r) = (
            Snowflake::from_u64(5),
            Snowflake::from_u64(6),
            Snowflake::from_u64(7),
        );
        let create = CreateEmoji {
            name: "wave",
            image: "data:image/png;base64,AA==",
            roles: Some(&[r]),
            ..Default::default()
        };
        let modify = ModifyEmoji {
            roles: Some(None),
            ..Default::default()
        };
        block_on(c.list_guild_emojis(g)).unwrap();
        block_on(c.get_guild_emoji(g, e)).unwrap();
        block_on(c.create_guild_emoji(g, &create)).unwrap();
        block_on(c.modify_guild_emoji(g, e, &modify)).unwrap();
        block_on(c.delete_guild_emoji(g, e, None)).unwrap();
        block_on(c.list_application_emojis()).unwrap();
        block_on(c.get_application_emoji(e)).unwrap();
        block_on(c.create_application_emoji(&create)).unwrap();
        block_on(c.modify_application_emoji(e, &modify)).unwrap();
        block_on(c.delete_application_emoji(e)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/guilds/5/emojis".into()),
                (Get, "/guilds/5/emojis/6".into()),
                (Post, "/guilds/5/emojis".into()),
                (Patch, "/guilds/5/emojis/6".into()),
                (Delete, "/guilds/5/emojis/6".into()),
                (Get, "/applications/1/emojis".into()),
                (Get, "/applications/1/emojis/6".into()),
                (Post, "/applications/1/emojis".into()),
                (Patch, "/applications/1/emojis/6".into()),
                (Delete, "/applications/1/emojis/6".into()),
            ]
        );
        assert_eq!(
            t.body(2),
            json!({ "name": "wave", "image": "data:image/png;base64,AA==", "roles": ["7"] })
        );
        assert_eq!(t.body(3), json!({ "roles": null }));
    }

    #[test]
    fn emojis_with_role_ids_deserialize() {
        let emoji: Emoji = serde_json::from_value(json!({
            "id": "1", "name": "wave", "roles": ["7"], "require_colons": true,
            "managed": false, "animated": false, "available": true,
        }))
        .unwrap();
        assert_eq!(emoji.roles.unwrap()[0].to_u64(), 7);
    }
}
