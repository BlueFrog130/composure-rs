use serde::{Deserialize, Serialize};

use crate::models::{
    common::Snowflake,
    deserialize::{Role, User},
};

#[derive(Debug, Deserialize, Serialize)]
pub struct PartialEmoji {
    /// [emoji id](https://discord.com/developers/docs/reference#image-formatting)
    pub id: Option<Snowflake>,

    /// emoji name
    pub name: Option<String>,

    /// whether this emoji is animated
    pub animated: Option<bool>,
}

/// [Emoji Object](https://discord.com/developers/docs/resources/emoji#emoji-object)
#[derive(Debug, Deserialize)]
pub struct Emoji {
    /// [emoji id](https://discord.com/developers/docs/reference#image-formatting)
    pub id: Option<Snowflake>,

    /// emoji name
    pub name: Option<String>,

    /// roles allowed to use this emoji
    pub roles: Option<Vec<Role>>,

    /// user that created this emoji
    pub user: Option<User>,

    /// whether this emoji must be wrapped in colons
    pub require_colons: Option<bool>,

    /// whether this emoji is managed
    pub managed: Option<bool>,

    /// whether this emoji is animated
    pub animated: Option<bool>,

    /// whether this emoji can be used, may be false due to loss of Server Boosts
    pub available: Option<bool>,
}

impl PartialEq for Emoji {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
