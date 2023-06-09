use composure::models::{Permissions, Snowflake, TypeField};
use serde::Deserialize;
use serde_json::Value;

use crate::command::*;

impl ApplicationCommand {
    pub fn new_chat_input_command(
        name: String,
        description: String,
        default_member_permissions: Option<Permissions>,
        dm_permission: Option<bool>,
        nsfw: Option<bool>,
        options: Option<Vec<ApplicationCommandOption>>,
    ) -> ApplicationCommand {
        ApplicationCommand::ChatInputCommand(ChatInputCommand {
            details: CommandDetails {
                t: TypeField,
                id: None,
                application_id: None,
                guild_id: None,
                name,
                name_localizations: None,
                default_member_permissions,
                dm_permission,
                nsfw,
                version: None,
            },
            options,
            description,
            description_localizations: None,
        })
    }

    pub fn new_user_command(
        name: String,
        default_member_permissions: Option<Permissions>,
        dm_permission: Option<bool>,
        nsfw: Option<bool>,
    ) -> ApplicationCommand {
        ApplicationCommand::UserCommand(CommandDetails {
            t: TypeField,
            id: None,
            application_id: None,
            guild_id: None,
            name,
            name_localizations: None,
            default_member_permissions,
            dm_permission,
            nsfw,
            version: None,
        })
    }

    pub fn new_message_command(
        name: String,
        default_member_permissions: Option<Permissions>,
        dm_permission: Option<bool>,
        nsfw: Option<bool>,
    ) -> ApplicationCommand {
        ApplicationCommand::MessageCommand(CommandDetails {
            t: TypeField,
            id: None,
            application_id: None,
            guild_id: None,
            name,
            name_localizations: None,
            default_member_permissions,
            dm_permission,
            nsfw,
            version: None,
        })
    }

    pub fn get_guild_id(&self) -> &Option<Snowflake> {
        match self {
            ApplicationCommand::ChatInputCommand(value) => &value.details.guild_id,
            ApplicationCommand::UserCommand(value) => &value.guild_id,
            ApplicationCommand::MessageCommand(value) => &value.guild_id,
        }
    }
}

impl<'de> Deserialize<'de> for ApplicationCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        let t = value
            .get("type")
            .and_then(Value::as_u64)
            .ok_or(serde::de::Error::missing_field("type"))?;

        match t {
            1 => Ok(ApplicationCommand::ChatInputCommand(
                ChatInputCommand::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            2 => Ok(ApplicationCommand::UserCommand(
                CommandDetails::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            3 => Ok(ApplicationCommand::MessageCommand(
                CommandDetails::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            _ => Err(serde::de::Error::custom("Unknown command")),
        }
    }
}

impl ApplicationCommandOption {
    pub fn new_subcommand_option(
        name: String,
        description: String,
        options: Option<Vec<SubcommandCommandOption>>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::Subcommand(SubcommandOption {
            name,
            description,
            options,
            t: TypeField::<1>,
            name_localizations: None,
            description_localizations: None,
        })
    }

    pub fn new_subcommand_group_option(
        name: String,
        description: String,
        options: Option<Vec<SubcommandOption>>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::SubcommandGroup(SubcommandGroupOption {
            name,
            description,
            options,
            t: TypeField::<2>,
            name_localizations: None,
            description_localizations: None,
        })
    }

    pub fn new_string_option(
        name: String,
        description: String,
        required: Option<bool>,
        choices: Option<Vec<ApplicationCommandOptionChoice<String>>>,
        min_length: Option<i32>,
        max_length: Option<i32>,
        autocomplete: Option<bool>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::String(StringOption {
            name,
            description,
            required,
            choices,
            min_length,
            max_length,
            autocomplete,
            t: TypeField::<3>,
            name_localizations: None,
            description_localizations: None,
        })
    }

    pub fn new_integer_option(
        name: String,
        description: String,
        required: Option<bool>,
        choices: Option<Vec<ApplicationCommandOptionChoice<i64>>>,
        min_value: Option<i64>,
        max_value: Option<i64>,
        autocomplete: Option<bool>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::Integer(IntegerOption {
            name,
            description,
            required,
            choices,
            min_value,
            max_value,
            autocomplete,
            t: TypeField::<4>,
            name_localizations: None,
            description_localizations: None,
        })
    }

    pub fn new_boolean_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::Boolean(Self::new_base_option::<5>(name, description, required))
    }

    pub fn new_user_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::User(Self::new_base_option::<6>(name, description, required))
    }

    pub fn new_channel_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::Channel(Self::new_base_option::<7>(name, description, required))
    }

    pub fn new_role_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::Role(Self::new_base_option::<8>(name, description, required))
    }

    pub fn new_mentionable_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::Mentionable(Self::new_base_option::<9>(
            name,
            description,
            required,
        ))
    }

    pub fn new_attachment_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::Attachment(Self::new_base_option::<11>(
            name,
            description,
            required,
        ))
    }

    pub fn new_number_option(
        name: String,
        description: String,
        required: Option<bool>,
        choices: Option<Vec<ApplicationCommandOptionChoice<f64>>>,
        min_value: Option<f64>,
        max_value: Option<f64>,
        autocomplete: Option<bool>,
    ) -> ApplicationCommandOption {
        ApplicationCommandOption::Number(NumberOption {
            name,
            description,
            required,
            choices,
            min_value,
            max_value,
            autocomplete,
            t: TypeField::<10>,
            name_localizations: None,
            description_localizations: None,
        })
    }

    fn new_base_option<const T: u8>(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> BaseOption<T> {
        BaseOption {
            t: TypeField::<T>,
            name,
            name_localizations: None,
            description,
            description_localizations: None,
            required,
        }
    }
}

impl<'de> Deserialize<'de> for ApplicationCommandOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        let t = value
            .get("type")
            .and_then(Value::as_u64)
            .ok_or(serde::de::Error::missing_field("type"))?;

        match t {
            1 => Ok(ApplicationCommandOption::Subcommand(
                SubcommandOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            2 => Ok(ApplicationCommandOption::SubcommandGroup(
                SubcommandGroupOption::deserialize(value)
                    .map_err(|e| serde::de::Error::custom(e))?,
            )),
            3 => Ok(ApplicationCommandOption::String(
                StringOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            4 => Ok(ApplicationCommandOption::Integer(
                IntegerOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            5 => Ok(ApplicationCommandOption::Boolean(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            6 => Ok(ApplicationCommandOption::User(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            7 => Ok(ApplicationCommandOption::Channel(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            8 => Ok(ApplicationCommandOption::Role(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            9 => Ok(ApplicationCommandOption::Mentionable(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            10 => Ok(ApplicationCommandOption::Number(
                NumberOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            11 => Ok(ApplicationCommandOption::Attachment(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            _ => Err(serde::de::Error::custom("Unknown option")),
        }
    }
}

impl SubcommandCommandOption {
    pub fn new_string_option(
        name: String,
        description: String,
        required: Option<bool>,
        choices: Option<Vec<ApplicationCommandOptionChoice<String>>>,
        min_length: Option<i32>,
        max_length: Option<i32>,
        autocomplete: Option<bool>,
    ) -> SubcommandCommandOption {
        SubcommandCommandOption::String(StringOption {
            name,
            description,
            required,
            choices,
            min_length,
            max_length,
            autocomplete,
            t: TypeField::<3>,
            name_localizations: None,
            description_localizations: None,
        })
    }

    pub fn new_integer_option(
        name: String,
        description: String,
        required: Option<bool>,
        choices: Option<Vec<ApplicationCommandOptionChoice<i64>>>,
        min_value: Option<i64>,
        max_value: Option<i64>,
        autocomplete: Option<bool>,
    ) -> SubcommandCommandOption {
        SubcommandCommandOption::Integer(IntegerOption {
            name,
            description,
            required,
            choices,
            min_value,
            max_value,
            autocomplete,
            t: TypeField::<4>,
            name_localizations: None,
            description_localizations: None,
        })
    }

    pub fn new_boolean_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> SubcommandCommandOption {
        SubcommandCommandOption::Boolean(Self::new_base_option::<5>(name, description, required))
    }

    pub fn new_user_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> SubcommandCommandOption {
        SubcommandCommandOption::User(Self::new_base_option::<6>(name, description, required))
    }

    pub fn new_channel_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> SubcommandCommandOption {
        SubcommandCommandOption::Channel(Self::new_base_option::<7>(name, description, required))
    }

    pub fn new_role_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> SubcommandCommandOption {
        SubcommandCommandOption::Role(Self::new_base_option::<8>(name, description, required))
    }

    pub fn new_mentionable_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> SubcommandCommandOption {
        SubcommandCommandOption::Mentionable(Self::new_base_option::<9>(
            name,
            description,
            required,
        ))
    }

    pub fn new_attachment_option(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> SubcommandCommandOption {
        SubcommandCommandOption::Attachment(Self::new_base_option::<11>(
            name,
            description,
            required,
        ))
    }

    pub fn new_number_option(
        name: String,
        description: String,
        required: Option<bool>,
        choices: Option<Vec<ApplicationCommandOptionChoice<f64>>>,
        min_value: Option<f64>,
        max_value: Option<f64>,
        autocomplete: Option<bool>,
    ) -> SubcommandCommandOption {
        SubcommandCommandOption::Number(NumberOption {
            name,
            description,
            required,
            choices,
            min_value,
            max_value,
            autocomplete,
            t: TypeField::<10>,
            name_localizations: None,
            description_localizations: None,
        })
    }

    fn new_base_option<const T: u8>(
        name: String,
        description: String,
        required: Option<bool>,
    ) -> BaseOption<T> {
        BaseOption {
            t: TypeField::<T>,
            name,
            name_localizations: None,
            description,
            description_localizations: None,
            required,
        }
    }
}

impl TryFrom<ApplicationCommandOption> for SubcommandCommandOption {
    type Error = ();

    fn try_from(value: ApplicationCommandOption) -> Result<Self, Self::Error> {
        match value {
            ApplicationCommandOption::Subcommand(_) => Err(()),
            ApplicationCommandOption::SubcommandGroup(_) => Err(()),
            ApplicationCommandOption::String(x) => Ok(SubcommandCommandOption::String(x)),
            ApplicationCommandOption::Integer(x) => Ok(SubcommandCommandOption::Integer(x)),
            ApplicationCommandOption::Boolean(x) => Ok(SubcommandCommandOption::Boolean(x)),
            ApplicationCommandOption::User(x) => Ok(SubcommandCommandOption::User(x)),
            ApplicationCommandOption::Channel(x) => Ok(SubcommandCommandOption::Channel(x)),
            ApplicationCommandOption::Role(x) => Ok(SubcommandCommandOption::Role(x)),
            ApplicationCommandOption::Mentionable(x) => Ok(SubcommandCommandOption::Mentionable(x)),
            ApplicationCommandOption::Number(x) => Ok(SubcommandCommandOption::Number(x)),
            ApplicationCommandOption::Attachment(x) => Ok(SubcommandCommandOption::Attachment(x)),
        }
    }
}

impl<'de> Deserialize<'de> for SubcommandCommandOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        let t = value
            .get("type")
            .and_then(Value::as_u64)
            .ok_or(serde::de::Error::missing_field("type"))?;

        match t {
            3 => Ok(SubcommandCommandOption::String(
                StringOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            4 => Ok(SubcommandCommandOption::Integer(
                IntegerOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            5 => Ok(SubcommandCommandOption::Boolean(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            6 => Ok(SubcommandCommandOption::User(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            7 => Ok(SubcommandCommandOption::Channel(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            8 => Ok(SubcommandCommandOption::Role(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            9 => Ok(SubcommandCommandOption::Mentionable(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            10 => Ok(SubcommandCommandOption::Number(
                NumberOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            11 => Ok(SubcommandCommandOption::Attachment(
                BaseOption::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            _ => Err(serde::de::Error::custom("Unknown option")),
        }
    }
}
