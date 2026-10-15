use serde::Deserialize;

use crate::{
    models::common::{Permissions, Snowflake},
    Mentionable,
};

/// User object
#[derive(Debug, Deserialize)]
pub struct User {
    /// User's [avatar hash](https://discord.com/developers/docs/reference#image-formatting)
    pub avatar: Option<String>,

    /// User's 4 digit discord tag
    pub discriminator: String,

    /// User's display name
    pub display_name: Option<String>,

    /// User Id
    pub id: Snowflake,

    /// Public [flags](https://discord.com/developers/docs/resources/user#user-object-user-flags) on a user's account
    pub public_flags: u64,

    /// Users name - not unique
    pub username: String,
}

impl Mentionable for User {
    fn to_mention(&self) -> String {
        format!("<@{}>", self.id)
    }
}

#[derive(Debug, Deserialize)]
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
#[derive(Debug, Deserialize)]
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

impl Mentionable for Member {
    fn to_mention(&self) -> String {
        format!("<@{}>", self.user.id)
    }
}
