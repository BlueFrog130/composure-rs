use std::collections::HashMap;

use serde::{Deserialize, Deserializer, de, de::DeserializeOwned};
use serde_json::{Value, value::RawValue};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::{
    ApplicationIntegrationType, Attachment, Channel, Entitlement, InteractionContextType, Member,
    Message, PartialChannel, PartialMember, Permissions, Role, Snowflake, User,
};

pub type ApplicationCommandInteraction = DataInteraction<ApplicationCommandInteractionData>;
pub type MessageComponentInteraction = DataInteraction<MessageComponentData>;
pub type ModalSubmitInteraction = DataInteraction<ModalSubmitData>;

/// [Interaction Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-interaction-structure)
#[derive(Debug, Clone)]
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
        let envelope = Envelope::deserialize(deserializer)?;
        let common = InteractionCommon {
            id: envelope.id,
            application_id: envelope.application_id,
            guild_id: envelope.guild_id,
            channel: envelope.channel,
            channel_id: envelope.channel_id,
            member: envelope.member,
            user: envelope.user,
            token: envelope.token,
            version: envelope.version,
            message: envelope.message,
            app_permissions: envelope.app_permissions,
            guild_locale: envelope.guild_locale,
            entitlements: envelope.entitlements,
            authorizing_integration_owners: envelope.authorizing_integration_owners,
            context: envelope.context,
        };
        let (locale, data) = (envelope.locale, envelope.data);

        Ok(match envelope.t {
            1 => Interaction::Ping(PingInteraction { common }),
            2 => Interaction::ApplicationCommand(DataInteraction::new(common, locale, data)?),
            3 => Interaction::MessageComponent(DataInteraction::new(common, locale, data)?),
            4 => Interaction::ApplicationCommandAutocomplete(DataInteraction::new(
                common, locale, data,
            )?),
            5 => Interaction::ModalSubmit(DataInteraction::new(common, locale, data)?),
            _ => return Err(de::Error::custom("unknown interaction type")),
        })
    }
}

/// An Interaction read in one pass: the common fields typed, and `data` kept as raw JSON until
/// `type` says what it is (Discord does not send `type` first)
#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "type")]
    t: u8,
    locale: Option<String>,
    data: Option<Box<RawValue>>,

    id: Snowflake,
    application_id: Snowflake,
    guild_id: Option<Snowflake>,
    channel: Option<Channel>,
    channel_id: Option<Snowflake>,
    member: Option<Member>,
    user: Option<User>,
    token: String,
    version: u8,
    message: Option<Box<Message>>,
    app_permissions: Option<Permissions>,
    guild_locale: Option<String>,
    #[serde(default)]
    entitlements: Vec<Entitlement>,
    #[serde(default, deserialize_with = "deserialize_integration_owners")]
    authorizing_integration_owners: HashMap<ApplicationIntegrationType, Snowflake>,
    context: Option<InteractionContextType>,
}

#[derive(Debug, Clone, Deserialize)]
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

    /// User object for the invoking user, if invoked in a DM. Use [`InteractionCommon::invoking_user`] to get the user in any context.
    pub user: Option<User>,

    /// Continuation token for responding to the interaction
    pub token: String,

    /// Read-only property, always 1
    pub version: u8,

    /// For components, the message they are attached to
    pub message: Option<Box<Message>>,

    /// Bitwise set of permissions the app or bot has within the channel the interaction was sent from
    pub app_permissions: Option<Permissions>,

    /// [Guild's preferred locale](https://discord.com/developers/docs/resources/guild#guild-object), if invoked in a guild
    pub guild_locale: Option<String>,

    /// Installation contexts the interaction is authorized for, mapped to the guild id (guild install;
    /// `"0"` in a DM with the bot) or the authorizing user's id (user install)
    #[serde(default, deserialize_with = "deserialize_integration_owners")]
    pub authorizing_integration_owners: HashMap<ApplicationIntegrationType, Snowflake>,

    /// Context the interaction was triggered from
    pub context: Option<InteractionContextType>,

    /// [Entitlements](https://discord.com/developers/docs/resources/entitlement) of the invoking
    /// user and guild for the app's premium SKUs
    #[serde(default)]
    pub entitlements: Vec<Entitlement>,
}

/// Keys are integration types as strings (`"0"`, `"1"`); unknown types are skipped.
fn deserialize_integration_owners<'de, D>(
    deserializer: D,
) -> Result<HashMap<ApplicationIntegrationType, Snowflake>, D::Error>
where
    D: Deserializer<'de>,
{
    let owners = HashMap::<String, Snowflake>::deserialize(deserializer)?;

    Ok(owners
        .into_iter()
        .filter_map(|(key, owner)| {
            let key = key.parse::<u8>().ok()?;
            let integration_type =
                ApplicationIntegrationType::deserialize(Value::from(key)).ok()?;
            Some((integration_type, owner))
        })
        .collect())
}

impl InteractionCommon {
    /// The user who invoked the interaction: `member.user` in a guild, `user` in DMs and private channels.
    /// `None` only for pings.
    pub fn invoking_user(&self) -> Option<&User> {
        self.member
            .as_ref()
            .map(|member| &member.user)
            .or(self.user.as_ref())
    }
}

#[derive(Debug, Clone)]
pub struct PingInteraction {
    pub common: InteractionCommon,
}

#[derive(Debug, Clone)]
pub struct DataInteraction<D> {
    pub common: InteractionCommon,
    pub locale: Option<String>,
    pub data: D,
}

impl<D: DeserializeOwned> DataInteraction<D> {
    fn new<E: de::Error>(
        common: InteractionCommon,
        locale: Option<String>,
        data: Option<Box<RawValue>>,
    ) -> Result<Self, E> {
        let data = data.ok_or_else(|| E::missing_field("data"))?;
        Ok(Self {
            common,
            locale,
            data: serde_json::from_str(data.get()).map_err(E::custom)?,
        })
    }
}

/// [Interaction Data](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-interaction-data)
#[derive(Debug, Clone, Deserialize)]
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

    pub fn resolved_channel(&self, snowflake: &Snowflake) -> Option<&PartialChannel> {
        self.resolved
            .as_ref()
            .and_then(|r| r.channels.as_ref())
            .and_then(|c| c.get(snowflake))
    }

    pub fn resolved_message(&self, snowflake: &Snowflake) -> Option<&Message> {
        self.resolved
            .as_ref()
            .and_then(|r| r.messages.as_ref())
            .and_then(|m| m.get(snowflake))
    }

    pub fn resolved_attachment(&self, snowflake: &Snowflake) -> Option<&Attachment> {
        self.resolved
            .as_ref()
            .and_then(|r| r.attachments.as_ref())
            .and_then(|a| a.get(snowflake))
    }

    pub fn first_option(&self) -> Option<&ApplicationCommandInteractionDataOption> {
        self.options.as_ref().and_then(|o| o.single())
    }

    /// The option the user is typing in, for autocomplete interactions
    pub fn focused_option(&self) -> Option<&FocusedOption> {
        self.options.as_ref().and_then(|o| o.focused())
    }
}

/// [Message Component Data Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-message-component-data-structure)
#[derive(Debug, Clone, Deserialize)]
pub struct MessageComponentData {
    /// the [custom_id](https://discord.com/developers/docs/interactions/message-components#custom-id) of the component
    pub custom_id: String,

    /// the [type](https://discord.com/developers/docs/interactions/message-components#component-object-component-types) of the component
    pub component_type: MessageComponentType,

    /// values the user selected in a [select menu](https://discord.com/developers/docs/interactions/message-components#select-menu-object) component:
    /// option values for string selects, ids for user/role/mentionable/channel selects
    pub values: Option<Vec<String>>,

    /// resolved entities for user, role, mentionable, and channel selects
    pub resolved: Option<ResolvedData>,
}

#[derive(Debug, Clone, Deserialize_repr)]
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
#[derive(Debug, Clone, Deserialize)]
pub struct ModalSubmitData {
    /// the [custom_id](https://discord.com/developers/docs/interactions/message-components#custom-id) of the modal
    pub custom_id: String,

    /// the values submitted by the user
    pub components: Vec<ModalSubmitComponent>,
}

impl ModalSubmitData {
    /// The submitted value of the text input with `custom_id`, wherever it is nested
    pub fn value(&self, custom_id: &str) -> Option<&str> {
        self.find(custom_id).and_then(|c| c.value.as_deref())
    }

    /// The submitted values of the select with `custom_id`, wherever it is nested
    pub fn values(&self, custom_id: &str) -> Option<&[String]> {
        self.find(custom_id).and_then(|c| c.values.as_deref())
    }

    /// The component with `custom_id`, wherever it is nested
    pub fn find(&self, custom_id: &str) -> Option<&ModalSubmitComponent> {
        self.components.iter().find_map(|c| c.find(custom_id))
    }
}

/// A component in a [modal submission](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-modal-submit-data-structure):
/// an action row or label wrapping inputs, or an input carrying the submitted value
#[derive(Debug, Clone, Deserialize)]
pub struct ModalSubmitComponent {
    /// [component type](https://discord.com/developers/docs/components/reference#component-object-component-types)
    #[serde(rename = "type")]
    pub t: u8,

    /// Developer-defined id of an input
    pub custom_id: Option<String>,

    /// Submitted text of a text input
    pub value: Option<String>,

    /// Submitted values of a select
    pub values: Option<Vec<String>>,

    /// Children of an action row
    pub components: Option<Vec<ModalSubmitComponent>>,

    /// Child of a label
    pub component: Option<Box<ModalSubmitComponent>>,
}

impl ModalSubmitComponent {
    fn find(&self, custom_id: &str) -> Option<&ModalSubmitComponent> {
        if self.custom_id.as_deref() == Some(custom_id) {
            return Some(self);
        }
        self.components
            .iter()
            .flatten()
            .chain(self.component.as_deref())
            .find_map(|c| c.find(custom_id))
    }
}

/// [Resolved Data Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-resolved-data-structure)
#[derive(Debug, Clone, Deserialize)]
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
#[derive(Debug, Clone)]
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
    /// Value is the attachment id; see [`ApplicationCommandInteractionData::resolved_attachment`]
    Attachment(SnowflakeOption),
    /// The option being typed in an autocomplete interaction, whatever its declared type
    Focused(FocusedOption),
}

/// The option a user is typing in an autocomplete interaction. Discord sends the partial input,
/// which for integer and number options may not parse yet, so it is kept as text.
#[derive(Debug, Clone)]
pub struct FocusedOption {
    pub name: String,

    /// [Option type](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-option-type)
    pub t: u8,

    /// What the user has typed so far
    pub value: String,
}

impl ApplicationCommandInteractionDataOption {
    pub fn name(&self) -> &str {
        match self {
            Self::Subcommand(s) => &s.name,
            Self::SubcommandGroup(s) => &s.name,
            Self::String(s) => &s.name,
            Self::Integer(s) => &s.name,
            Self::Boolean(s) => &s.name,
            Self::User(s) | Self::Channel(s) | Self::Role(s) | Self::Mentionable(s) => &s.name,
            Self::Number(s) => &s.name,
            Self::Attachment(s) => &s.name,
            Self::Focused(s) => &s.name,
        }
    }
}

impl<'de> Deserialize<'de> for ApplicationCommandInteractionDataOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use ApplicationCommandInteractionDataOption as O;
        use RawOptionValue as V;

        let RawOption {
            name,
            t,
            value,
            options,
            focused,
        } = RawOption::deserialize(deserializer)?;

        if focused == Some(true) && t > 2 {
            let value = match value {
                Some(V::String(s)) => s,
                Some(V::Integer(i)) => i.to_string(),
                Some(V::Number(n)) => n.to_string(),
                Some(V::Boolean(b)) => b.to_string(),
                None => String::new(),
            };
            return Ok(O::Focused(FocusedOption { name, t, value }));
        }

        Ok(match (t, value) {
            (1, _) => O::Subcommand(Subcommand {
                name,
                options: OptionList(options.unwrap_or_default()),
                focused,
            }),
            (2, _) => match options.and_then(|o| o.into_iter().next()) {
                Some(O::Subcommand(subcommand)) => O::SubcommandGroup(SubcommandGroup {
                    name,
                    subcommand,
                    focused,
                }),
                _ => return Err(de::Error::custom("subcommand group without a subcommand")),
            },
            (3, Some(V::String(s))) => O::String(ValueOption::new(name, s, focused)),
            (4, Some(V::Integer(i))) => O::Integer(ValueOption::new(name, i, focused)),
            (5, Some(V::Boolean(b))) => O::Boolean(ValueOption::new(name, b, focused)),
            (10, Some(V::Number(n))) => O::Number(ValueOption::new(name, n, focused)),
            // Discord sends whole numbers without a fraction
            (10, Some(V::Integer(i))) => O::Number(ValueOption::new(name, i as f64, focused)),
            (6..=9 | 11, Some(V::String(s))) => {
                let Ok(id) = s.parse() else {
                    return Err(de::Error::custom(format_args!(
                        "option `{name}`: invalid id"
                    )));
                };
                let option = ValueOption::new(name, id, focused);
                match t {
                    6 => O::User(option),
                    7 => O::Channel(option),
                    8 => O::Role(option),
                    9 => O::Mentionable(option),
                    _ => O::Attachment(option),
                }
            }
            (3..=11, _) => {
                return Err(de::Error::custom(format_args!(
                    "option `{name}`: value does not match type {t}"
                )));
            }
            _ => return Err(de::Error::custom("unknown option type")),
        })
    }
}

/// An option read in one pass; turned into the variant its `type` names
#[derive(Deserialize)]
struct RawOption {
    name: String,
    #[serde(rename = "type")]
    t: u8,
    value: Option<RawOptionValue>,
    options: Option<Vec<ApplicationCommandInteractionDataOption>>,
    focused: Option<bool>,
}

/// An option `value` as JSON sent it
enum RawOptionValue {
    String(String),
    Integer(i64),
    Number(f64),
    Boolean(bool),
}

impl<'de> Deserialize<'de> for RawOptionValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = RawOptionValue;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string, number, or boolean")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(RawOptionValue::String(v.to_string()))
            }

            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(RawOptionValue::String(v))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(RawOptionValue::Integer(v))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                i64::try_from(v)
                    .map(RawOptionValue::Integer)
                    .map_err(|_| E::invalid_value(de::Unexpected::Unsigned(v), &self))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(RawOptionValue::Number(v))
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(RawOptionValue::Boolean(v))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

#[derive(Debug, Clone, Default)]
pub struct OptionList(Vec<ApplicationCommandInteractionDataOption>);

impl OptionList {
    pub const fn empty() -> Self {
        OptionList(Vec::new())
    }

    pub fn iter(&self) -> impl Iterator<Item = &ApplicationCommandInteractionDataOption> {
        self.0.iter()
    }

    /// The option being typed in an autocomplete interaction, searching into subcommands
    pub fn focused(&self) -> Option<&FocusedOption> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::Focused(f) => Some(f),
            ApplicationCommandInteractionDataOption::Subcommand(s) => s.options.focused(),
            ApplicationCommandInteractionDataOption::SubcommandGroup(g) => {
                g.subcommand.options.focused()
            }
            _ => None,
        })
    }

    pub fn get_number_option(&self, name: &str) -> Option<&NumberOption> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::Number(s) if s.name == name => Some(s),
            _ => None,
        })
    }

    pub fn get_attachment_option(&self, name: &str) -> Option<&SnowflakeOption> {
        self.0.iter().find_map(|o| match o {
            ApplicationCommandInteractionDataOption::Attachment(s) if s.name == name => Some(s),
            _ => None,
        })
    }

    pub fn single(&self) -> Option<&ApplicationCommandInteractionDataOption> {
        self.0.first()
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
        self.0.iter().find(|o| o.name() == name)
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

#[derive(Debug, Clone, Deserialize)]
pub struct Subcommand {
    /// Name of the parameter
    pub name: String,

    /// Present if this option is a group or subcommand
    #[serde(default)]
    pub options: OptionList,

    /// true if this option is the currently focused option for autocomplete
    pub focused: Option<bool>,
}

#[derive(Debug, Clone)]
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
            options,
        } = InnerData::deserialize(deserializer)?;

        match options.into_iter().next() {
            Some(ApplicationCommandInteractionDataOption::Subcommand(subcommand)) => {
                Ok(SubcommandGroup {
                    name,
                    subcommand,
                    focused,
                })
            }
            _ => Err(serde::de::Error::custom("Not a subcommand")),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValueOption<T> {
    /// Name of the parameter
    pub name: String,

    /// Value of the option resulting from user input
    pub value: T,

    /// true if this option is the currently focused option for autocomplete
    pub focused: Option<bool>,
}

impl<T> ValueOption<T> {
    fn new(name: String, value: T, focused: Option<bool>) -> Self {
        Self {
            name,
            value,
            focused,
        }
    }
}

/// [Application Command Types](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
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
#[derive(Debug, Clone, Deserialize)]
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

    #[test]
    pub fn current_format_interaction() {
        let json = r#"{
            "app_permissions": "2248473465835073",
            "application_id": "1052322265397739523",
            "attachment_size_limit": 10485760,
            "authorizing_integration_owners": { "0": "798662131062931547" },
            "channel": {
                "flags": 0,
                "guild_id": "798662131062931547",
                "id": "941169456686723122",
                "last_message_id": "1100155827400229026",
                "name": "bot-stuff",
                "nsfw": false,
                "parent_id": "798662131678969866",
                "permissions": "2251799813685247",
                "position": 1,
                "rate_limit_per_user": 0,
                "topic": null,
                "type": 0
            },
            "channel_id": "941169456686723122",
            "context": 0,
            "data": {
                "guild_id": "798662131062931547",
                "id": "1052358444704862218",
                "name": "ping",
                "type": 1
            },
            "entitlement_sku_ids": [],
            "entitlements": [],
            "guild": { "features": [], "id": "798662131062931547", "locale": "en-US" },
            "guild_id": "798662131062931547",
            "guild_locale": "en-US",
            "id": "1100173248714518568",
            "locale": "en-US",
            "member": {
                "avatar": null,
                "banner": null,
                "collectibles": null,
                "communication_disabled_until": null,
                "deaf": false,
                "flags": 0,
                "joined_at": "2021-01-12T21:18:10.481000+00:00",
                "mute": false,
                "nick": null,
                "pending": false,
                "permissions": "2251799813685247",
                "premium_since": null,
                "roles": ["943607715639484456"],
                "unusual_dm_activity_until": null,
                "user": {
                    "avatar": "fa82e15e24ee16c9fcbf8dd34d10b4cc",
                    "avatar_decoration_data": null,
                    "clan": null,
                    "collectibles": null,
                    "discriminator": "0",
                    "global_name": "Blue Frog",
                    "id": "282265607313817601",
                    "primary_guild": null,
                    "public_flags": 0,
                    "username": "bluefrog"
                }
            },
            "token": "A_UNIQUE_TOKEN",
            "type": 2,
            "version": 1
        }"#;

        let Interaction::ApplicationCommand(interaction) =
            serde_json::from_str::<Interaction>(json).unwrap()
        else {
            panic!("expected application command");
        };

        let member = interaction.common.member.unwrap();
        assert_eq!(member.user.discriminator, "0");
        assert_eq!(member.user.global_name.as_deref(), Some("Blue Frog"));
        assert_eq!(member.display_name(), "Blue Frog");
    }

    fn user_install_interaction(context: u8, location: &str) -> ApplicationCommandInteraction {
        let json = format!(
            r#"{{
                "app_permissions": "562949953601536",
                "application_id": "1052322265397739523",
                "attachment_size_limit": 10485760,
                "authorizing_integration_owners": {{ "1": "282265607313817601" }},
                "context": {context},
                "data": {{ "id": "1052358444704862218", "name": "ping", "type": 1 }},
                "entitlements": [],
                "id": "1100173248714518568",
                "locale": "en-US",
                {location},
                "token": "A_UNIQUE_TOKEN",
                "type": 2,
                "version": 1
            }}"#
        );

        match serde_json::from_str::<Interaction>(&json).unwrap() {
            Interaction::ApplicationCommand(interaction) => interaction,
            other => panic!("expected application command, got {other:?}"),
        }
    }

    const USER: &str = r#"{
        "avatar": null,
        "discriminator": "0",
        "global_name": "Blue Frog",
        "id": "282265607313817601",
        "public_flags": 0,
        "username": "bluefrog"
    }"#;

    #[test]
    pub fn user_install_guild_context() {
        let location = format!(
            r#""guild_id": "798662131062931547",
            "guild_locale": "en-US",
            "channel_id": "941169456686723122",
            "member": {{
                "avatar": null,
                "deaf": false,
                "flags": 0,
                "joined_at": "2021-01-12T21:18:10.481000+00:00",
                "mute": false,
                "nick": "Froggy",
                "pending": false,
                "permissions": "2251799813685247",
                "premium_since": null,
                "roles": [],
                "user": {USER}
            }}"#
        );
        let interaction = user_install_interaction(0, &location);
        let common = &interaction.common;

        assert_eq!(common.context, Some(InteractionContextType::Guild));
        assert_eq!(
            common.authorizing_integration_owners[&ApplicationIntegrationType::UserInstall],
            Snowflake::from_u64(282265607313817601)
        );
        assert!(common.user.is_none());
        assert_eq!(
            common.invoking_user().unwrap().id,
            Snowflake::from_u64(282265607313817601)
        );
    }

    #[test]
    pub fn bot_dm_context() {
        let location = format!(
            r#""channel": {{
                "flags": 0,
                "id": "1100155827400229026",
                "last_message_id": null,
                "recipients": [{USER}],
                "type": 1
            }},
            "channel_id": "1100155827400229026",
            "user": {USER}"#
        );
        let interaction = user_install_interaction(1, &location);
        let common = &interaction.common;

        assert_eq!(common.context, Some(InteractionContextType::BotDm));
        assert!(common.member.is_none());
        assert_eq!(common.invoking_user().unwrap().username, "bluefrog");
    }

    #[test]
    pub fn private_channel_context() {
        let location = format!(
            r#""channel": {{ "id": "1100155827400229026", "type": 3 }},
            "channel_id": "1100155827400229026",
            "user": {USER}"#
        );
        let interaction = user_install_interaction(2, &location);
        let common = &interaction.common;

        assert_eq!(common.context, Some(InteractionContextType::PrivateChannel));
        assert!(common.guild_id.is_none());
        assert!(common.member.is_none());
        assert_eq!(common.invoking_user().unwrap().display_name(), "Blue Frog");
    }

    #[test]
    pub fn guild_install_owners() {
        let json = r#"{
            "application_id": "1052322265397739523",
            "authorizing_integration_owners": { "0": "798662131062931547", "1": "282265607313817601" },
            "context": 0,
            "id": "1100173248714518568",
            "token": "A_UNIQUE_TOKEN",
            "type": 1,
            "version": 1
        }"#;

        let Interaction::Ping(ping) = serde_json::from_str::<Interaction>(json).unwrap() else {
            panic!("expected ping");
        };

        let owners = &ping.common.authorizing_integration_owners;
        assert_eq!(
            owners[&ApplicationIntegrationType::GuildInstall],
            Snowflake::from_u64(798662131062931547)
        );
        assert_eq!(
            owners[&ApplicationIntegrationType::UserInstall],
            Snowflake::from_u64(282265607313817601)
        );
        assert!(ping.common.invoking_user().is_none());
    }
}
