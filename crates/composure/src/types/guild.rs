use serde::Deserialize;

use crate::types::Snowflake;

/// The commonly sent fields of a [Guild](https://discord.com/developers/docs/resources/guild#guild-object)
#[derive(Debug, Clone, Deserialize)]
pub struct PartialGuild {
    pub id: Snowflake,

    pub name: String,

    /// [Icon hash](https://discord.com/developers/docs/reference#image-formatting)
    pub icon: Option<String>,

    pub owner_id: Option<Snowflake>,

    /// Enabled [guild features](https://discord.com/developers/docs/resources/guild#guild-object-guild-features)
    #[serde(default)]
    pub features: Vec<String>,
}
