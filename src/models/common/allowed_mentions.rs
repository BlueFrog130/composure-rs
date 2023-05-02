use serde::{Deserialize, Serialize};

use crate::models::Snowflake;

/// [Allowed Mentions Structure](https://discord.comundefinedhttps://discord.com/developers/docs/resources/channel#allowed-mentions-object-allowed-mentions-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct AllowedMentions {
    /// An array of [allowed mention types](https://discord.com/developers/docs/resources/channel#allowed-mentions-object-allowed-mention-types) to parse from the content.
    pub parse: Vec<AllowedMentionTypes>,

    /// Array of role_ids to mention (Max size of 100)
    pub roles: Vec<Snowflake>,

    /// Array of user_ids to mention (Max size of 100)
    pub users: Vec<Snowflake>,

    /// For replies, whether to mention the author of the message being replied to (default false)
    pub replied_user: bool,
}

/// [Allowed Mention Types](https://discord.com/developers/docs/resources/channel#allowed-mentions-object-allowed-mention-types)
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AllowedMentionTypes {
    Roles,
    Users,
    Everyone,
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn serializes() {
        let allowed_mentions = AllowedMentions {
            parse: vec![AllowedMentionTypes::Roles],
            roles: vec![],
            users: vec![],
            replied_user: false,
        };

        println!(
            "{}",
            serde_json::to_string_pretty(&allowed_mentions).unwrap()
        );
    }
}
