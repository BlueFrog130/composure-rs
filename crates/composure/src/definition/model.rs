use std::collections::HashMap;

use crate::types::{
    ApplicationIntegrationType, ChannelType, InteractionContextType, Permissions, Snowflake,
    TypeField,
};
use serde::{Deserialize, Serialize};

/// [Application Command Structure](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-structure)
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ApplicationCommand {
    /// A slash Command (type 1)
    ChatInputCommand(ChatInputCommand<1>),
    /// A user context menu Command (type 2)
    UserCommand(CommandDetails<2>),
    /// A message context menu Command (type 3)
    MessageCommand(CommandDetails<3>),
}

impl ApplicationCommand {
    /// The slash Command, if it is one
    pub fn as_chat_input_command(&self) -> Option<&ChatInputCommand<1>> {
        if let Self::ChatInputCommand(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// The user Command, if it is one
    pub fn as_user_command(&self) -> Option<&CommandDetails<2>> {
        if let Self::UserCommand(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// The message Command, if it is one
    pub fn as_message_command(&self) -> Option<&CommandDetails<3>> {
        if let Self::MessageCommand(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// What every Command Definition has: name, permissions, contexts, ... `T` is the Command type;
/// user and message Commands are only this
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDetails<const T: u8> {
    /// The Command type, `T`
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
    ///
    /// Deprecated by Discord: use [`CommandDetails::contexts`] instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dm_permission: Option<bool>,

    /// [Installation contexts](https://discord.com/developers/docs/resources/application#installation-context) where the command is available, only for globally-scoped commands. Defaults to the app's configured contexts
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integration_types: Option<Vec<ApplicationIntegrationType>>,

    /// [Interaction contexts](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-interaction-context-types) where the command can be used, only for globally-scoped commands
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<InteractionContextType>>,

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

/// A slash Command's Definition: [`CommandDetails`] plus a description and Options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatInputCommand<const T: u8> {
    /// Name, permissions, contexts, ...
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

/// A boolean Option's Definition
pub type BooleanOption = BaseOption<5>;
/// A user Option's Definition
pub type UserOption = BaseOption<6>;
/// A role Option's Definition
pub type RoleOption = BaseOption<8>;
/// A mentionable (user or role) Option's Definition
pub type MentionableOption = BaseOption<9>;
/// An attachment Option's Definition
pub type AttachmentOption = BaseOption<11>;

/// [Application Command Option Structure](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-option-structure)
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ApplicationCommandOption {
    /// A subcommand
    Subcommand(SubcommandOption),
    /// A group of subcommands
    SubcommandGroup(SubcommandGroupOption),
    /// A string Option
    String(StringOption),
    /// An integer Option
    Integer(IntegerOption),
    /// A boolean Option
    Boolean(BooleanOption),
    /// A user Option
    User(UserOption),
    /// A channel Option
    Channel(ChannelOption),
    /// A role Option
    Role(RoleOption),
    /// A mentionable Option
    Mentionable(MentionableOption),
    /// A number Option
    Number(NumberOption),
    /// An attachment Option
    Attachment(AttachmentOption),
}

/// Subcommand options
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SubcommandCommandOption {
    /// A string Option
    String(StringOption),
    /// An integer Option
    Integer(IntegerOption),
    /// A boolean Option
    Boolean(BooleanOption),
    /// A user Option
    User(UserOption),
    /// A channel Option
    Channel(ChannelOption),
    /// A role Option
    Role(RoleOption),
    /// A mentionable Option
    Mentionable(MentionableOption),
    /// A number Option
    Number(NumberOption),
    /// An attachment Option
    Attachment(AttachmentOption),
}

/// A subcommand's Definition: a name, description and Options of its own
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubcommandOption {
    /// The Option type, always `1`
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SubcommandCommandOption>>,
}

/// A subcommand group's Definition: subcommands under one name
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubcommandGroupOption {
    /// The Option type, always `2`
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SubcommandOption>>,
}

/// A string Option's Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringOption {
    /// The Option type, always `3`
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

/// An integer Option's Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegerOption {
    /// The Option type, always `4`
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

/// A number (`f64`) Option's Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberOption {
    /// The Option type, always `10`
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

/// The Definition of an Option without settings of its own (boolean, user, role, mentionable,
/// attachment); `T` is the Option type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseOption<const T: u8> {
    /// The Option type, `T`
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

/// A channel Option's Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelOption {
    /// The Option type, always `7`
    #[serde(rename = "type")]
    pub t: TypeField<7>,

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

    /// The channel types the option is restricted to; any channel type if unset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_types: Option<Vec<ChannelType>>,
}

/// [Application Command Option Choice Structure](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-option-choice-structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationCommandOptionChoice<T> {
    /// 1-100 character choice name
    pub name: String,

    /// Localization dictionary for the name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// Value for the choice, up to 100 characters if string
    pub value: T,
}
