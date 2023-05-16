use std::collections::HashMap;

use composure::models::{Permissions, Snowflake, TypeField};
use serde::{Deserialize, Serialize};

/// [Application Command Structure](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-structure)
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ApplicationCommand {
    ChatInputCommand(ChatInputCommand<1>),
    UserCommand(CommandDetails<2>),
    MessageCommand(CommandDetails<3>),
}

impl ApplicationCommand {
    pub fn as_chat_input_command(&self) -> Option<&ChatInputCommand<1>> {
        if let Self::ChatInputCommand(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_user_command(&self) -> Option<&CommandDetails<2>> {
        if let Self::UserCommand(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_message_command(&self) -> Option<&CommandDetails<3>> {
        if let Self::MessageCommand(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandDetails<const T: u8> {
    #[serde(rename = "type")]
    pub t: TypeField<T>,

    /// Unique ID of command
    #[serde(skip_serializing)]
    pub id: Option<Snowflake>,

    /// ID of the parent application
    #[serde(skip_serializing)]
    pub application_id: Option<Snowflake>,

    /// Guild ID of the command, if not global
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,

    /// [Name of command](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-naming), 1-32 characters
    pub name: String,

    /// Localization dictionary for name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// Set of [permissions](https://discord.com/developers/docs/topics/permissions) represented as a bit set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_member_permissions: Option<Permissions>,

    /// Indicates whether the command is available in DMs with the app, only for globally-scoped commands. By default, commands are visible.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dm_permission: Option<bool>,

    /// Not recommended for use as field will soon be deprecated. Indicates whether the command is enabled by default when the app is added to a guild, defaults to true
    // #[serde(skip_serializing_if = "Option::is_none")]
    // pub default_permission: Option<bool>,

    /// Indicates whether the command is [age-restricted](https://discord.com/developers/docs/interactions/application-commands#agerestricted-commands), defaults to false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,

    /// Autoincrementing version identifier updated during substantial record changes
    #[serde(skip_serializing)]
    pub version: Option<Snowflake>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatInputCommand<const T: u8> {
    #[serde(flatten)]
    pub details: CommandDetails<T>,

    /// Description for CHAT_INPUT commands, 1-100 characters. Empty string for USER and MESSAGE commands
    pub description: String,

    /// Localization dictionary for description field. Values follow the same restrictions as description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,

    /// Parameters for the command, max of 25
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ApplicationCommandOption>>,
}

pub type BooleanOption = BaseOption<5>;
pub type UserOption = BaseOption<6>;
pub type ChannelOption = BaseOption<7>;
pub type RoleOption = BaseOption<8>;
pub type MentionableOption = BaseOption<9>;
pub type AttachmentOption = BaseOption<11>;

/// [Application Command Option Structure](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-option-structure)
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ApplicationCommandOption {
    Subcommand(SubcommandOption),
    SubcommandGroup(SubcommandGroupOption),
    String(StringOption),
    Integer(IntegerOption),
    Boolean(BooleanOption),
    User(UserOption),
    Channel(ChannelOption),
    Role(RoleOption),
    Mentionable(MentionableOption),
    Number(NumberOption),
    Attachment(AttachmentOption),
}

/// Subcommand options
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum SubcommandCommandOption {
    String(StringOption),
    Integer(IntegerOption),
    Boolean(BooleanOption),
    User(UserOption),
    Channel(ChannelOption),
    Role(RoleOption),
    Mentionable(MentionableOption),
    Number(NumberOption),
    Attachment(AttachmentOption),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubcommandOption {
    #[serde(rename = "type")]
    pub t: TypeField<1>,

    /// [1-32 character name](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-naming)
    pub name: String,

    /// Localization dictionary for the name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// 1-100 character description
    pub description: String,

    /// Localization dictionary for the description field. Values follow the same restrictions as description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,

    /// If the option is a subcommand or subcommand group type, these nested options will be the parameters
    pub options: Option<Vec<SubcommandCommandOption>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubcommandGroupOption {
    #[serde(rename = "type")]
    pub t: TypeField<2>,

    /// [1-32 character name](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-naming)
    pub name: String,

    /// Localization dictionary for the name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// 1-100 character description
    pub description: String,

    /// Localization dictionary for the description field. Values follow the same restrictions as description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,

    /// If the option is a subcommand or subcommand group type, these nested options will be the parameters
    pub options: Option<Vec<SubcommandOption>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StringOption {
    #[serde(rename = "type")]
    pub t: TypeField<3>,

    /// [1-32 character name](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-naming)
    pub name: String,

    /// Localization dictionary for the name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// 1-100 character description
    pub description: String,

    /// Localization dictionary for the description field. Values follow the same restrictions as description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,

    /// If the parameter is required or optional--default false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Choices for STRING, INTEGER, and NUMBER types for the user to pick from, max 25
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<ApplicationCommandOptionChoice<String>>>,

    /// For option type STRING, the minimum allowed length (minimum of 0, maximum of 6000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<i32>,

    /// For option type STRING, the maximum allowed length (minimum of 1, maximum of 6000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i32>,

    /// If autocomplete interactions are enabled for this STRING, INTEGER, or NUMBER type option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocomplete: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IntegerOption {
    #[serde(rename = "type")]
    pub t: TypeField<4>,

    /// [1-32 character name](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-naming)
    pub name: String,

    /// Localization dictionary for the name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// 1-100 character description
    pub description: String,

    /// Localization dictionary for the description field. Values follow the same restrictions as description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,

    /// If the parameter is required or optional--default false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Choices for STRING, INTEGER, and NUMBER types for the user to pick from, max 25
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<ApplicationCommandOptionChoice<i64>>>,

    /// If the option is an INTEGER or NUMBER type, the minimum value permitted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<i64>,

    /// If the option is an INTEGER or NUMBER type, the maximum value permitted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<i64>,

    /// If autocomplete interactions are enabled for this STRING, INTEGER, or NUMBER type option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocomplete: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NumberOption {
    #[serde(rename = "type")]
    pub t: TypeField<10>,

    /// [1-32 character name](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-naming)
    pub name: String,

    /// Localization dictionary for the name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// 1-100 character description
    pub description: String,

    /// Localization dictionary for the description field. Values follow the same restrictions as description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,

    /// If the parameter is required or optional--default false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Choices for STRING, INTEGER, and NUMBER types for the user to pick from, max 25
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<ApplicationCommandOptionChoice<f64>>>,

    /// If the option is an INTEGER or NUMBER type, the minimum value permitted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,

    /// If the option is an INTEGER or NUMBER type, the maximum value permitted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,

    /// If autocomplete interactions are enabled for this STRING, INTEGER, or NUMBER type option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocomplete: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BaseOption<const T: u8> {
    #[serde(rename = "type")]
    pub t: TypeField<T>,

    /// [1-32 character name](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-naming)
    pub name: String,

    /// Localization dictionary for the name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// 1-100 character description
    pub description: String,

    /// Localization dictionary for the description field. Values follow the same restrictions as description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,

    /// If the parameter is required or optional--default false
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// [Application Command Option Choice Structure](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-option-choice-structure)
#[derive(Debug, Serialize, Deserialize)]
pub struct ApplicationCommandOptionChoice<T> {
    /// 1-100 character choice name
    pub name: String,

    /// Localization dictionary for the name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// Value for the choice, up to 100 characters if string
    pub value: Vec<T>,
}
