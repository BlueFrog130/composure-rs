use std::collections::HashMap;

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::models::{
    ActionRow, Attachment, Channel, Member, Message, PartialChannel, PartialMember, Permissions,
    Role, SelectOption, Snowflake, User,
};

pub type ApplicationCommandInteraction = DataInteraction<ApplicationCommandInteractionData>;
pub type MessageComponentInteraction = DataInteraction<MessageComponentData>;
pub type ModalSubmitInteraction = DataInteraction<ModalSubmitData>;

/// [Interaction Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-interaction-structure)
#[derive(Debug)]
pub enum Interaction {
    Ping(PingInteraction),
    ApplicationCommand(ApplicationCommandInteraction),
    MessageComponent(MessageComponentInteraction),
    ApplicationCommandAutocomplete(ApplicationCommandInteraction),
    ModalSubmit(ModalSubmitInteraction),
}

impl<'de> Deserialize<'de> for Interaction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        let t = value
            .get("type")
            .and_then(Value::as_u64)
            .ok_or(serde::de::Error::missing_field("type"))?;

        match t {
            // Ping
            1 => Ok(Interaction::Ping(
                PingInteraction::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            // Application Command
            2 => Ok(Interaction::ApplicationCommand(
                DataInteraction::<ApplicationCommandInteractionData>::deserialize(value)
                    .map_err(|e| serde::de::Error::custom(e))?,
            )),
            // Message Component
            3 => Ok(Interaction::MessageComponent(
                DataInteraction::<MessageComponentData>::deserialize(value)
                    .map_err(|e| serde::de::Error::custom(e))?,
            )),
            // Autocomplete
            4 => Ok(Interaction::ApplicationCommandAutocomplete(
                DataInteraction::<ApplicationCommandInteractionData>::deserialize(value)
                    .map_err(|e| serde::de::Error::custom(e))?,
            )),
            // Modal Submit
            5 => Ok(Interaction::ModalSubmit(
                DataInteraction::<ModalSubmitData>::deserialize(value)
                    .map_err(|e| serde::de::Error::custom(e))?,
            )),
            _ => Err(serde::de::Error::custom("Unknown interaction")),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct InteractionCommon {
    /// ID of the interaction
    pub id: Snowflake,

    /// ID of the application this interaction is for
    pub application_id: Snowflake,

    /// Guild that the interaction was sent from
    pub guild_id: Option<Snowflake>,

    /// Channel that the interaction was sent from
    pub channel: Option<Channel>,

    /// Channel that the interaction was sent from
    pub channel_id: Option<Snowflake>,

    /// Guild member data for the invoking user, including permissions
    pub member: Option<Member>,

    /// User object for the invoking user, if invoked in a DM
    pub user: Option<User>,

    /// Continuation token for responding to the interaction
    pub token: String,

    /// Read-only property, always 1
    pub version: u8,

    /// For components, the message they were attached to
    // pub message: Option<>,

    /// Bitwise set of permissions the app or bot has within the channel the interaction was sent from
    pub app_permissions: Option<Permissions>,

    /// [Guild's preferred locale](https://discord.com/developers/docs/resources/guild#guild-object), if invoked in a guild
    pub guild_locale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PingInteraction {
    #[serde(flatten)]
    pub common: InteractionCommon,
}

#[derive(Debug, Deserialize)]
pub struct DataInteraction<D> {
    #[serde(flatten)]
    pub common: InteractionCommon,
    pub locale: Option<String>,
    pub data: D,
}

/// [Interaction Data](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-interaction-data)
#[derive(Debug, Deserialize)]
pub struct ApplicationCommandInteractionData {
    /// the [ID](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-structure) of the invoked command
    pub id: Snowflake,

    /// the [name](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-structure) of the invoked command
    pub name: String,

    /// the [type](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-structure) of the invoked command
    #[serde(rename = "type")]
    pub t: ApplicationCommandType,

    /// converted users + roles + channels + attachments
    pub resolved: Option<ResolvedData>,

    /// the params + values from the user
    pub options: Option<OptionList>,

    /// the id of the guild the command is registered to
    pub guild_id: Option<Snowflake>,

    /// id of the [user](https://discord.com/developers/docs/interactions/application-commands#user-commands) or [message](https://discord.com/developers/docs/interactions/application-commands#message-commands) targeted by a user or message command
    pub target_id: Option<Snowflake>,
}

impl ApplicationCommandInteractionData {
    pub fn resolved_user(&self, snowflake: &Snowflake) -> Option<&User> {
        self.resolved
            .as_ref()
            .and_then(|r| r.users.as_ref())
            .and_then(|u| u.get(snowflake))
    }

    pub fn resolved_member(&self, snowflake: &Snowflake) -> Option<&PartialMember> {
        self.resolved
            .as_ref()
            .and_then(|r| r.members.as_ref())
            .and_then(|u| u.get(snowflake))
    }

    pub fn resolved_role(&self, snowflake: &Snowflake) -> Option<&Role> {
        self.resolved
            .as_ref()
            .and_then(|r| r.roles.as_ref())
            .and_then(|u| u.get(snowflake))
    }

    pub fn first_option(&self) -> Option<&ApplicationCommandInteractionDataOption> {
        self.options.as_ref().and_then(|o| o.single())
    }
}

/// [Message Component Data Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-message-component-data-structure)
#[derive(Debug, Deserialize)]
pub struct MessageComponentData {
    /// the [custom_id](https://discord.com/developers/docs/interactions/message-components#custom-id) of the component
    pub custom_id: String,

    /// the [type](https://discord.com/developers/docs/interactions/message-components#component-object-component-types) of the component
    pub component_type: MessageComponentType,

    /// values the user selected in a [select menu](https://discord.com/developers/docs/interactions/message-components#select-menu-object) component
    pub values: Option<Vec<SelectOption>>,
}

#[derive(Debug, Deserialize_repr)]
#[repr(u8)]
pub enum MessageComponentType {
    ActionRow = 1,
    Button = 2,
    StringSelect = 3,
    TextInput = 4,
    UserSelect = 5,
    RoleSelect = 6,
    MentionableSelect = 7,
    ChannelSelect = 8,
}

/// [Modal Submit Data Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-modal-submit-data-structure)
#[derive(Debug, Deserialize)]
pub struct ModalSubmitData {
    /// the [custom_id](https://discord.com/developers/docs/interactions/message-components#custom-id) of the modal
    pub custom_id: String,

    /// the values submitted by the user
    pub components: Vec<ActionRow>, // TODO: this is a guess - might need to be a Vec<Component>
}

/// [Resolved Data Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-resolved-data-structure)
#[derive(Debug, Deserialize)]
pub struct ResolvedData {
    /// the ids and User objects
    pub users: Option<HashMap<Snowflake, User>>,

    /// the ids and partial Member objects
    pub members: Option<HashMap<Snowflake, PartialMember>>,

    /// the ids and Role objects
    pub roles: Option<HashMap<Snowflake, Role>>,

    /// the ids and partial Channel objects
    pub channels: Option<HashMap<Snowflake, PartialChannel>>,

    /// the ids and partial Message objects
    pub messages: Option<HashMap<Snowflake, Message>>,

    /// the ids and attachment objects
    pub attachments: Option<HashMap<Snowflake, Attachment>>,
}

pub type StringOption = ValueOption<String>;
pub type IntegerOption = ValueOption<i64>;
pub type BooleanOption = ValueOption<bool>;
pub type SnowflakeOption = ValueOption<Snowflake>;
pub type NumberOption = ValueOption<f64>;

/// [Application Command Interaction Data Option Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-application-command-interaction-data-option-structure)
#[derive(Debug)]
pub enum ApplicationCommandInteractionDataOption {
    Subcommand(Subcommand),
    SubcommandGroup(SubcommandGroup),
    String(StringOption),
    Integer(IntegerOption),
    Boolean(BooleanOption),
    User(SnowflakeOption),
    Channel(SnowflakeOption),
    Role(SnowflakeOption),
    Mentionable(SnowflakeOption),
    Number(NumberOption),
    Attachment, // TODO: Figure out value type
}

impl<'de> Deserialize<'de> for ApplicationCommandInteractionDataOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        let t = value
            .get("type")
            .and_then(Value::as_u64)
            .ok_or(serde::de::Error::missing_field("type"))?;

        match t {
            1 => Ok(ApplicationCommandInteractionDataOption::Subcommand(
                Subcommand::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            2 => Ok(ApplicationCommandInteractionDataOption::SubcommandGroup(
                SubcommandGroup::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            3 => Ok(ApplicationCommandInteractionDataOption::String(
                ValueOption::<String>::deserialize(value)
                    .map_err(|e| serde::de::Error::custom(e))?,
            )),
            4 => Ok(ApplicationCommandInteractionDataOption::Integer(
                ValueOption::<i64>::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            5 => Ok(ApplicationCommandInteractionDataOption::Boolean(
                ValueOption::<bool>::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            6 => Ok(ApplicationCommandInteractionDataOption::User(
                ValueOption::<Snowflake>::deserialize(value)
                    .map_err(|e| serde::de::Error::custom(e))?,
            )),
            7 => Ok(ApplicationCommandInteractionDataOption::Channel(
                ValueOption::<Snowflake>::deserialize(value)
                    .map_err(|e| serde::de::Error::custom(e))?,
            )),
            8 => Ok(ApplicationCommandInteractionDataOption::Role(
                ValueOption::<Snowflake>::deserialize(value)
                    .map_err(|e| serde::de::Error::custom(e))?,
            )),
            9 => Ok(ApplicationCommandInteractionDataOption::Mentionable(
                ValueOption::<Snowflake>::deserialize(value)
                    .map_err(|e| serde::de::Error::custom(e))?,
            )),
            10 => Ok(ApplicationCommandInteractionDataOption::Number(
                ValueOption::<f64>::deserialize(value).map_err(|e| serde::de::Error::custom(e))?,
            )),
            11 => Ok(ApplicationCommandInteractionDataOption::Attachment),
            _ => Err(serde::de::Error::custom("Unknown option")),
        }
    }
}

#[derive(Debug)]
pub struct OptionList(Vec<ApplicationCommandInteractionDataOption>);

impl OptionList {
    pub fn single(&self) -> Option<&ApplicationCommandInteractionDataOption> {
        self.0.get(0)
    }

    pub fn subcommand(&self) -> Option<&Subcommand> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::Subcommand(s) => Some(s),
            _ => None,
        })
    }

    pub fn subcommand_group(&self) -> Option<&SubcommandGroup> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::SubcommandGroup(s) => Some(s),
            _ => None,
        })
    }

    pub fn get_option(&self, name: &str) -> Option<&ApplicationCommandInteractionDataOption> {
        self.0.iter().find(|o| match o {
            ApplicationCommandInteractionDataOption::Subcommand(s) => s.name == name,
            ApplicationCommandInteractionDataOption::SubcommandGroup(s) => s.name == name,
            ApplicationCommandInteractionDataOption::String(s) => s.name == name,
            ApplicationCommandInteractionDataOption::Integer(s) => s.name == name,
            ApplicationCommandInteractionDataOption::Boolean(s) => s.name == name,
            ApplicationCommandInteractionDataOption::User(s) => s.name == name,
            ApplicationCommandInteractionDataOption::Channel(s) => s.name == name,
            ApplicationCommandInteractionDataOption::Role(s) => s.name == name,
            ApplicationCommandInteractionDataOption::Mentionable(s) => s.name == name,
            ApplicationCommandInteractionDataOption::Number(s) => s.name == name,
            ApplicationCommandInteractionDataOption::Attachment => false,
        })
    }

    pub fn get_string_option(&self, name: &str) -> Option<&StringOption> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::String(s) if s.name == name => Some(s),
            _ => None,
        })
    }

    pub fn get_integer_option(&self, name: &str) -> Option<&IntegerOption> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::Integer(s) if s.name == name => Some(s),
            _ => None,
        })
    }

    pub fn get_boolean_option(&self, name: &str) -> Option<&BooleanOption> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::Boolean(s) if s.name == name => Some(s),
            _ => None,
        })
    }

    pub fn get_user_option(&self, name: &str) -> Option<&SnowflakeOption> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::User(s) if s.name == name => Some(s),
            _ => None,
        })
    }

    pub fn get_channel_option(&self, name: &str) -> Option<&SnowflakeOption> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::Channel(s) if s.name == name => Some(s),
            _ => None,
        })
    }

    pub fn get_role_option(&self, name: &str) -> Option<&SnowflakeOption> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::Role(s) if s.name == name => Some(s),
            _ => None,
        })
    }

    pub fn get_mentionable_option(&self, name: &str) -> Option<&SnowflakeOption> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::Mentionable(s) if s.name == name => Some(s),
            _ => None,
        })
    }
}

impl<'de> Deserialize<'de> for OptionList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(OptionList(
            Vec::<ApplicationCommandInteractionDataOption>::deserialize(deserializer)?,
        ))
    }
}

#[derive(Debug, Deserialize)]
pub struct Subcommand {
    /// Name of the parameter
    pub name: String,

    /// Present if this option is a group or subcommand
    pub options: OptionList,

    /// true if this option is the currently focused option for autocomplete
    pub focused: Option<bool>,
}

#[derive(Debug)]
pub struct SubcommandGroup {
    /// Name of the parameter
    pub name: String,

    /// The command being called
    pub subcommand: Subcommand,

    /// true if this option is the currently focused option for autocomplete
    pub focused: Option<bool>,
}

impl<'de> Deserialize<'de> for SubcommandGroup {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct InnerData {
            name: String,
            options: Vec<ApplicationCommandInteractionDataOption>,
            focused: Option<bool>,
        }

        let InnerData {
            name,
            focused,
            mut options,
        } = InnerData::deserialize(deserializer)?;

        let option = options.remove(0);

        match option {
            ApplicationCommandInteractionDataOption::Subcommand(subcommand) => {
                Ok(SubcommandGroup {
                    name,
                    subcommand,
                    focused,
                })
            }
            _ => return Err(serde::de::Error::custom("Not a subcommand")),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ValueOption<T> {
    /// Name of the parameter
    pub name: String,

    /// Value of the option resulting from user input
    pub value: T,

    /// true if this option is the currently focused option for autocomplete
    pub focused: Option<bool>,
}

/// [Application Command Types](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-types)
#[derive(Debug, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum ApplicationCommandType {
    /// Slash commands; a text-based command that shows up when a user types /
    ChatInput = 1,

    /// A UI-based command that shows up when you right click or tap on a user
    User = 2,

    /// A UI-based command that shows up when you right click or tap on a message
    Message = 3,
}

/// [Application Command Data](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-application-command-data-structure)
#[derive(Debug, Deserialize)]
pub struct InteractionData {
    /// the [ID](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-structure) of the invoked command
    pub id: Snowflake,

    /// the [name](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-structure) of the invoked command
    pub name: String,

    /// the [type](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-structure) of the invoked command
    #[serde(rename = "type")]
    pub t: ApplicationCommandType,

    /// converted users + roles + channels + attachments
    pub resolved: Option<ResolvedData>,

    /// the params + values from the user
    pub options: Option<Vec<ApplicationCommandInteractionDataOption>>,

    /// the id of the guild the command is registered to
    pub guild_id: Option<Snowflake>,

    /// id of the [user](https://discord.com/developers/docs/interactions/application-commands#user-commands) or [message](https://discord.com/developers/docs/interactions/application-commands#message-commands) targeted by a user or message command
    pub target_id: Option<Snowflake>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn ping_interaction() {
        let json = r#"{
            "application_id": "1052322265397739523",
			"type": 1,
			"token": "A_UNIQUE_TOKEN",
			"member": {
				"user": {
					"id": "53908232506183680",
					"username": "Mason",
					"avatar": "a_d5efa99b3eeaa7dd43acca82f5692432",
					"discriminator": "1337",
					"public_flags": 131141
				},
				"roles": ["539082325061836999"],
				"premium_since": null,
				"permissions": "2147483647",
				"pending": false,
				"nick": null,
				"mute": false,
				"joined_at": "2017-03-13T19:19:14.040000+00:00",
				"is_pending": false,
				"deaf": false,
                "flags": 0
			},
			"id": "786008729715212338",
			"guild_id": "290926798626357999",
			"app_permissions": "442368",
			"guild_locale": "en-US",
			"locale": "en-US",
			"channel_id": "645027906669510667",
            "version": 1
		}"#;

        let res = serde_json::from_str::<Interaction>(json);

        assert!(res.is_ok());

        let interaction = res.unwrap();

        assert!(matches!(
            interaction,
            Interaction::Ping(PingInteraction { .. })
        ))
    }

    #[test]
    pub fn command_interaction() {
        let json = r#"{
            "application_id": "1052322265397739523",
            "version": 1,
			"type": 2,
			"token": "A_UNIQUE_TOKEN",
			"member": {
				"user": {
					"id": "53908232506183680",
					"username": "Mason",
					"avatar": "a_d5efa99b3eeaa7dd43acca82f5692432",
					"discriminator": "1337",
					"public_flags": 131141
				},
				"roles": ["539082325061836999"],
				"premium_since": null,
				"permissions": "2147483647",
				"pending": false,
				"nick": null,
				"mute": false,
				"joined_at": "2017-03-13T19:19:14.040000+00:00",
				"is_pending": false,
				"deaf": false,
                "flags": 0
			},
			"id": "786008729715212338",
			"guild_id": "290926798626357999",
			"app_permissions": "442368",
			"guild_locale": "en-US",
			"locale": "en-US",
			"data": {
				"options": [{
					"type": 3,
					"name": "cardname",
					"value": "The Gitrog Monster"
				}],
				"type": 1,
				"name": "cardsearch",
				"id": "771825006014889984"
			},
			"channel_id": "645027906669510667"
		}"#;

        let res = serde_json::from_str::<Interaction>(json);

        assert!(res.is_ok());

        let interaction = res.unwrap();

        assert!(matches!(
            interaction,
            Interaction::ApplicationCommand(DataInteraction { .. })
        ))
    }

    #[test]
    pub fn real_interaction() {
        let json = r#"{
            "app_permissions": "137411140374081",
            "application_id": "1052322265397739523",
            "channel": {
                "flags": 0,
                "guild_id": "798662131062931547",
                "id": "941169456686723122",
                "last_message_id": "1100155827400229026",
                "name": "bot-stuff",
                "nsfw": false,
                "parent_id": "798662131678969866",
                "permissions": "140737488355327",
                "position": 1,
                "rate_limit_per_user": 0,
                "topic": null,
                "type": 0
            },
            "channel_id": "941169456686723122",
            "data": {
                "guild_id": "798662131062931547",
                "id": "1052358444704862218",
                "name": "ping",
                "type": 1
            },
            "entitlement_sku_ids": [],
            "entitlements": [],
            "guild_id": "798662131062931547",
            "guild_locale": "en-US",
            "id": "1100173248714518568",
            "locale": "en-US",
            "member": {
                "avatar": null,
                "communication_disabled_until": null,
                "deaf": false,
                "flags": 0,
                "is_pending": false,
                "joined_at": "2021-01-12T21:18:10.481000+00:00",
                "mute": false,
                "nick": null,
                "pending": false,
                "permissions": "140737488355327",
                "premium_since": null,
                "roles": [
                    "943607715639484456"
                ],
                "user": {
                    "avatar": "fa82e15e24ee16c9fcbf8dd34d10b4cc",
                    "avatar_decoration": null,
                    "discriminator": "9846",
                    "display_name": null,
                    "global_name": null,
                    "id": "282265607313817601",
                    "public_flags": 0,
                    "username": "BlueFrog"
                }
            },
            "token": "aW50ZXJhY3Rpb246MTEwMDE3MzI0ODcxNDUxODU2ODppVTFuSkNSbndrZ01Na3RCWk81MVhTWkdSbk8yTlBaM1U3Z3JlckR4YUZJMTZFTm9wc21nZnlaSnN4ZUZCTTd0Q0Jzc09ac3BHV1E1MGlBZGZnZzh0NDJmTElIcTB1M0FZQTJPS1BxcG1GTEtZUjNDWWFEamhEeTRPMWZnS0R4dQ",
            "type": 2,
            "version": 1
        }"#;

        let interaction = serde_json::from_str::<Interaction>(json);

        println!("{:#?}", interaction);

        assert!(interaction.is_ok());
    }
}
