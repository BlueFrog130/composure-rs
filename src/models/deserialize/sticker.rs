use serde::Deserialize;
use serde_repr::Deserialize_repr;

use crate::models::{Snowflake, User};

/// [Sticker Structure](https://discord.comundefinedhttps://discord.com/developers/docs/resources/sticker#sticker-object-sticker-structure)
#[derive(Debug, Deserialize)]
pub struct Sticker {
    /// [id of the sticker](https://discord.com/developers/docs/reference#image-formatting)
    pub id: Snowflake,

    /// for standard stickers, id of the pack the sticker is from
    pub pack_id: Option<Snowflake>,

    /// name of the sticker
    pub name: String,

    /// description of the sticker
    pub description: Option<String>,

    /// autocomplete/suggestion tags for the sticker (max 200 characters)
    pub tags: String,

    /// Deprecated previously the sticker asset hash, now an empty string
    pub asset: Option<String>,

    /// [type of sticker](https://discord.com/developers/docs/resources/sticker#sticker-object-sticker-types)
    #[serde(rename = "type")]
    pub t: StickerType,

    /// [type of sticker format](https://discord.com/developers/docs/resources/sticker#sticker-object-sticker-format-types)
    pub format_type: i32,

    /// whether this guild sticker can be used, may be false due to loss of Server Boosts
    pub available: Option<bool>,

    /// id of the guild that owns this sticker
    pub guild_id: Option<Snowflake>,

    /// the user that uploaded the guild sticker
    pub user: Option<User>,

    /// the standard sticker's sort order within its pack
    pub sort_value: Option<i32>,
}

/// [Sticker Types](https://discord.comundefinedhttps://discord.com/developers/docs/resources/sticker#sticker-object-sticker-types)
#[derive(Debug, Deserialize_repr)]
#[repr(u8)]

pub enum StickerType {
    /// an official sticker in a pack, part of Nitro or in a removed purchasable pack
    Standard = 1,

    /// a sticker uploaded to a guild for the guild's members
    Guild = 2,
}

/// [Sticker Format Types](https://discord.comundefinedhttps://discord.com/developers/docs/resources/sticker#sticker-object-sticker-format-types)
#[derive(Debug, Deserialize_repr)]
#[repr(u8)]

pub enum StickerFormatTypes {
    Png = 1,

    Apng = 2,

    Lottie = 3,

    Gif = 4,
}

/// [Sticker Item Structure](https://discord.comundefinedhttps://discord.com/developers/docs/resources/sticker#sticker-item-object-sticker-item-structure)
#[derive(Debug, Deserialize)]
pub struct StickerItem {
    /// id of the sticker
    pub id: Snowflake,

    /// name of the sticker
    pub name: String,

    /// [type of sticker format](https://discord.com/developers/docs/resources/sticker#sticker-object-sticker-format-types)
    pub format_type: i32,
}

/// [Sticker Pack Structure](https://discord.comundefinedhttps://discord.com/developers/docs/resources/sticker#sticker-pack-object-sticker-pack-structure)
#[derive(Debug, Deserialize)]
pub struct StickerPack {
    /// id of the sticker pack
    pub id: Snowflake,

    /// the stickers in the pack
    pub stickers: Vec<Sticker>,

    /// name of the sticker pack
    pub name: String,

    /// id of the pack's SKU
    pub sku_id: Snowflake,

    /// id of a sticker in the pack which is shown as the pack's icon
    pub cover_sticker_id: Option<Snowflake>,

    /// description of the sticker pack
    pub description: String,

    /// id of the sticker pack's [banner image](https://discord.com/developers/docs/reference#image-formatting)
    pub banner_asset_id: Option<Snowflake>,
}
