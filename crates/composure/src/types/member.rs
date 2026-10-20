use serde::Deserialize;

use crate::{
    Mentionable,
    types::{Permissions, Snowflake},
};

/// User object
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    /// User's [avatar hash](https://discord.com/developers/docs/reference#image-formatting)
    pub avatar: Option<String>,

    /// User's 4 digit discord tag; `"0"` for users migrated to unique usernames
    pub discriminator: String,

    /// User's display name, if set. Prefer [`User::display_name`] for showing a name.
    #[serde(default)]
    pub global_name: Option<String>,

    /// User Id
    pub id: Snowflake,

    /// Public [flags](https://discord.com/developers/docs/resources/user#user-object-user-flags) on a user's account
    #[serde(default)]
    pub public_flags: u64,

    /// Users name - not unique
    pub username: String,
}

impl User {
    /// Name to show for this user: global display name, falling back to username
    pub fn display_name(&self) -> &str {
        self.global_name.as_deref().unwrap_or(&self.username)
    }
}

impl Mentionable for User {
    fn to_mention(&self) -> String {
        format!("<@{}>", self.id)
    }
}

/// The member fields sent in resolved data: a [`Member`] without its `user`, which is resolved
/// separately
#[derive(Debug, Clone, Deserialize)]
pub struct PartialMember {
    /// Guild nickname
    pub nick: Option<String>,

    /// Guild member's [guild avatar hash](https://discord.com/developers/docs/reference#image-formatting)
    pub avatar: Option<String>,

    /// Array of role snowflakes
    pub roles: Vec<Snowflake>,

    /// When the user joined the guild formatted as ISO timestamp
    pub joined_at: String,

    /// When the user started boosting the guild
    pub premium_since: Option<String>,

    /// Whether the user has not yet passed the guild's [Membership Screening requirements](https://discord.com/developers/docs/resources/guild#membership-screening-object)
    pub pending: Option<bool>,

    /// total permissions of the member in the channel, including overwrites, returned when in the interaction object
    pub permissions: Permissions,
}

/// [Guild Member](https://discord.com/developers/docs/resources/guild#guild-member-object)
#[derive(Debug, Clone, Deserialize)]
pub struct Member {
    /// User this member represents
    pub user: User,

    /// Guild nickname
    pub nick: Option<String>,

    /// Guild member's [guild avatar hash](https://discord.com/developers/docs/reference#image-formatting)
    pub avatar: Option<String>,

    /// Array of role snowflakes
    pub roles: Vec<Snowflake>,

    /// When the user joined the guild formatted as ISO timestamp
    pub joined_at: String,

    /// When the user started boosting the guild
    pub premium_since: Option<String>,

    /// Whether the user is deafened in voice channels
    pub deaf: bool,

    /// Whether the user is muted in voice channels
    pub mute: bool,

    /// [Guild member flags](https://discord.com/developers/docs/resources/guild#guild-member-object-guild-member-flags)
    pub flags: u64,

    /// Whether the user has not yet passed the guild's [Membership Screening requirements](https://discord.com/developers/docs/resources/guild#membership-screening-object)
    pub pending: Option<bool>,

    /// total permissions of the member in the channel, including overwrites, returned when in the interaction object
    pub permissions: Option<Permissions>,

    /// when the user's [timeout](https://support.discord.com/hc/en-us/articles/4413305239191-Time-Out-FAQ) will expire and the user will be able to communicate in the guild again, null or a time in the past if the user is not timed out
    pub communication_disabled_until: Option<String>,
}

impl Member {
    /// Name to show for this member: server nickname, then global display name, then username
    pub fn display_name(&self) -> &str {
        self.nick
            .as_deref()
            .unwrap_or_else(|| self.user.display_name())
    }
}

impl Mentionable for Member {
    fn to_mention(&self) -> String {
        format!("<@{}>", self.user.id)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn user(global_name: Option<&str>) -> User {
        serde_json::from_value(json!({
            "id": "282265607313817601",
            "username": "bluefrog",
            "discriminator": "0",
            "global_name": global_name,
            "avatar": null,
        }))
        .unwrap()
    }

    fn member(nick: Option<&str>, global_name: Option<&str>) -> Member {
        Member {
            user: user(global_name),
            nick: nick.map(String::from),
            avatar: None,
            roles: Vec::new(),
            joined_at: String::from("2021-01-12T21:18:10.481000+00:00"),
            premium_since: None,
            deaf: false,
            mute: false,
            flags: 0,
            pending: None,
            permissions: None,
            communication_disabled_until: None,
        }
    }

    #[test]
    fn display_name_prefers_nick() {
        assert_eq!(member(Some("Nick"), Some("Global")).display_name(), "Nick");
    }

    #[test]
    fn display_name_falls_back_to_global_name() {
        assert_eq!(member(None, Some("Global")).display_name(), "Global");
        assert_eq!(user(Some("Global")).display_name(), "Global");
    }

    #[test]
    fn display_name_falls_back_to_username() {
        assert_eq!(member(None, None).display_name(), "bluefrog");
        assert_eq!(user(None).display_name(), "bluefrog");
    }

    #[test]
    fn legacy_user_deserializes() {
        let user: User = serde_json::from_value(json!({
            "id": "282265607313817601",
            "username": "BlueFrog",
            "discriminator": "9846",
            "avatar": "fa82e15e24ee16c9fcbf8dd34d10b4cc",
            "public_flags": 131141,
        }))
        .unwrap();

        assert_eq!(user.global_name, None);
        assert_eq!(user.public_flags, 131141);
        assert_eq!(user.display_name(), "BlueFrog");
    }
}
