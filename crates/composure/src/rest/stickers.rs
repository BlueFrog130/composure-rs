//! [Stickers](https://discord.com/developers/docs/resources/sticker#endpoints)

use serde::{Deserialize, Serialize};

use crate::types::{File, Snowflake, Sticker};

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    GET_STICKER = Get "/stickers/{sticker_id}";
    LIST_STICKER_PACKS = Get "/sticker-packs";
    LIST_GUILD_STICKERS = Get "/guilds/{guild_id}/stickers";
    GET_GUILD_STICKER = Get "/guilds/{guild_id}/stickers/{sticker_id}";
    CREATE_GUILD_STICKER = Post "/guilds/{guild_id}/stickers";
    MODIFY_GUILD_STICKER = Patch "/guilds/{guild_id}/stickers/{sticker_id}";
    DELETE_GUILD_STICKER = Delete "/guilds/{guild_id}/stickers/{sticker_id}";
}

/// Arguments of [`BotClient::create_guild_sticker`]
#[derive(Debug, Clone, Copy)]
pub struct CreateSticker<'a> {
    /// 2-30 characters
    pub name: &'a str,

    /// 2-100 characters (or empty)
    pub description: &'a str,

    /// Autocomplete/suggestion tags, comma separated (max 200 characters)
    pub tags: &'a str,

    /// PNG, APNG, GIF, or Lottie JSON (max 512 KiB)
    pub file: &'a File,

    /// `X-Audit-Log-Reason`
    pub reason: Option<&'a str>,
}

/// Body of [`BotClient::modify_guild_sticker`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModifySticker<'a> {
    /// 2-30 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,

    /// 2-100 characters; `Some(None)` removes it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<&'a str>>,

    /// Autocomplete/suggestion tags, up to 200 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<&'a str>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// [Sticker Pack](https://discord.com/developers/docs/resources/sticker#sticker-pack-object)
#[derive(Debug, Clone, Deserialize)]
pub struct StickerPack {
    /// The pack's id
    pub id: Snowflake,
    /// Its stickers
    pub stickers: Vec<Sticker>,
    /// Its name
    pub name: String,
    /// The SKU it belongs to
    pub sku_id: Snowflake,
    /// The sticker shown as the pack's icon
    pub cover_sticker_id: Option<Snowflake>,
    /// Its description
    pub description: String,
    /// Its banner image's id
    pub banner_asset_id: Option<Snowflake>,
}

/// The response of [`BotClient::list_sticker_packs`]
#[derive(Debug, Clone, Deserialize)]
pub struct StickerPacks {
    /// The packs
    pub sticker_packs: Vec<StickerPack>,
}

impl<T: Transport> BotClient<T> {
    /// [Get Sticker](https://discord.com/developers/docs/resources/sticker#get-sticker): standard
    /// or guild
    pub async fn get_sticker(&self, sticker_id: Snowflake) -> Result<Response<Sticker>> {
        let url = self.rest.url(GET_STICKER, &[&sticker_id]);
        self.rest.call(GET_STICKER, url, Body::NONE, None).await
    }

    /// [List Sticker Packs](https://discord.com/developers/docs/resources/sticker#list-sticker-packs)
    pub async fn list_sticker_packs(&self) -> Result<Response<StickerPacks>> {
        let url = self.rest.url(LIST_STICKER_PACKS, &[]);
        self.rest
            .call(LIST_STICKER_PACKS, url, Body::NONE, None)
            .await
    }

    /// [List Guild Stickers](https://discord.com/developers/docs/resources/sticker#list-guild-stickers)
    pub async fn list_guild_stickers(&self, guild_id: Snowflake) -> Result<Response<Vec<Sticker>>> {
        let url = self.rest.url(LIST_GUILD_STICKERS, &[&guild_id]);
        self.rest
            .call(LIST_GUILD_STICKERS, url, Body::NONE, None)
            .await
    }

    /// [Get Guild Sticker](https://discord.com/developers/docs/resources/sticker#get-guild-sticker)
    pub async fn get_guild_sticker(
        &self,
        guild_id: Snowflake,
        sticker_id: Snowflake,
    ) -> Result<Response<Sticker>> {
        let url = self.rest.url(GET_GUILD_STICKER, &[&guild_id, &sticker_id]);
        self.rest
            .call(GET_GUILD_STICKER, url, Body::NONE, None)
            .await
    }

    /// [Create Guild Sticker](https://discord.com/developers/docs/resources/sticker#create-guild-sticker)
    pub async fn create_guild_sticker(
        &self,
        guild_id: Snowflake,
        sticker: &CreateSticker<'_>,
    ) -> Result<Response<Sticker>> {
        let url = self.rest.url(CREATE_GUILD_STICKER, &[&guild_id]);
        let fields = [
            ("name", sticker.name),
            ("description", sticker.description),
            ("tags", sticker.tags),
        ];
        self.rest
            .call(
                CREATE_GUILD_STICKER,
                url,
                Body::<()>::Form(&fields, sticker.file),
                sticker.reason,
            )
            .await
    }

    /// [Modify Guild Sticker](https://discord.com/developers/docs/resources/sticker#modify-guild-sticker)
    pub async fn modify_guild_sticker(
        &self,
        guild_id: Snowflake,
        sticker_id: Snowflake,
        sticker: &ModifySticker<'_>,
    ) -> Result<Response<Sticker>> {
        let url = self
            .rest
            .url(MODIFY_GUILD_STICKER, &[&guild_id, &sticker_id]);
        self.rest
            .call(
                MODIFY_GUILD_STICKER,
                url,
                Body::Json(sticker),
                sticker.reason,
            )
            .await
    }

    /// [Delete Guild Sticker](https://discord.com/developers/docs/resources/sticker#delete-guild-sticker)
    pub async fn delete_guild_sticker(
        &self,
        guild_id: Snowflake,
        sticker_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self
            .rest
            .url(DELETE_GUILD_STICKER, &[&guild_id, &sticker_id]);
        self.rest
            .call_empty(DELETE_GUILD_STICKER, url, Body::NONE, reason)
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
        let (g, s) = (Snowflake::from_u64(5), Snowflake::from_u64(6));
        let file = File::new("wave.png", b"PNG".to_vec()).content_type("image/png");
        block_on(c.get_sticker(s)).unwrap();
        block_on(c.list_sticker_packs()).unwrap();
        block_on(c.list_guild_stickers(g)).unwrap();
        block_on(c.get_guild_sticker(g, s)).unwrap();
        block_on(c.create_guild_sticker(
            g,
            &CreateSticker {
                name: "wave",
                description: "hi",
                tags: "wave",
                file: &file,
                reason: Some("fun"),
            },
        ))
        .unwrap();
        block_on(c.modify_guild_sticker(
            g,
            s,
            &ModifySticker {
                description: Some(None),
                ..Default::default()
            },
        ))
        .unwrap();
        block_on(c.delete_guild_sticker(g, s, None)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/stickers/6".into()),
                (Get, "/sticker-packs".into()),
                (Get, "/guilds/5/stickers".into()),
                (Get, "/guilds/5/stickers/6".into()),
                (Post, "/guilds/5/stickers".into()),
                (Patch, "/guilds/5/stickers/6".into()),
                (Delete, "/guilds/5/stickers/6".into()),
            ]
        );
        let create = t.request(4);
        let body = String::from_utf8_lossy(create.body.as_deref().unwrap()).into_owned();
        assert!(body.contains("name=\"name\"\r\n\r\nwave\r\n"));
        assert!(body.contains("name=\"file\"; filename=\"wave.png\""));
        assert!(!body.contains("payload_json"));
        assert_eq!(create.header("x-audit-log-reason"), Some("fun"));
        assert_eq!(t.body(5), json!({ "description": null }));
    }
}
