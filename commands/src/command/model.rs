use std::collections::HashMap;

use interaction_bot::models::{Permissions, Snowflake, TypeField};
use serde::{Deserialize, Serialize};

/// [Application Command Structure](https://discord.comundefinedhttps://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-structure)
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ApplicationCommand {
    ChatInputCommand(ChatInputCommand<1>),
    UserCommand(CommandDetails<2>),
    MessageCommand(CommandDetails<3>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommandDetails<const T: u8> {
    #[serde(rename = "type")]
    pub t: TypeField<T>,

    /// Unique ID of command
    #[serde(skip_serializing)]
    pub id: Option<Snowflake>,

    /// ID of the parent application
    pub application_id: Snowflake,

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

/// [Application Command Option Structure](https://discord.comundefinedhttps://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-option-structure)
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ApplicationCommandOption {
    Subcommand(SubcommandOption<1>),
    SubcommandGroup(SubcommandOption<2>),
    String(StringOption<3>),
    Integer(IntegerOption<4>),
    Boolean(BaseOption<5>),
    User(BaseOption<6>),
    Channel(BaseOption<7>),
    Role(BaseOption<8>),
    Mentionable(BaseOption<9>),
    Number(NumberOption<10>),
    Attachment(BaseOption<11>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubcommandOption<const T: u8> {
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
    pub required: bool,

    /// If the option is a subcommand or subcommand group type, these nested options will be the parameters
    pub options: Vec<ApplicationCommandOption>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StringOption<const T: u8> {
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
    pub required: bool,

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
pub struct IntegerOption<const T: u8> {
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
    pub required: bool,

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
pub struct NumberOption<const T: u8> {
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
    pub required: bool,

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
    pub required: bool,
}

/// [Application Command Option Choice Structure](https://discord.comundefinedhttps://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-option-choice-structure)
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
