use std::fmt;

use crate::{
    definition::{
        ApplicationCommandOption, ApplicationCommandOptionChoice, BaseOption, ChannelOption,
        IntegerOption, NumberOption, StringOption,
    },
    types::{
        ApplicationCommandInteractionData, ApplicationCommandInteractionDataOption as DataOption,
        ApplicationCommandType, Attachment, ChannelType, Message, PartialChannel, PartialMember,
        Role, TypeField, User,
    },
};

/// Name, description, and constraints of one Option, from `#[option(...)]`
#[derive(Debug, Clone, Copy)]
pub struct OptionMeta {
    pub name: &'static str,
    pub description: &'static str,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub min_length: Option<i32>,
    pub max_length: Option<i32>,
    pub channel_types: Option<&'static [ChannelType]>,
}

impl OptionMeta {
    pub const fn new(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            description,
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            channel_types: None,
        }
    }
}

/// Why an Option value could not be read from an Interaction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionError {
    /// A required Option was not sent
    Missing,
    /// The Option was sent with a different type than defined
    WrongType,
    /// The referenced user, member, channel, role, attachment, or message is not in the resolved data
    Unresolved,
    /// The value does not fit the Rust type
    OutOfRange,
    /// The value is not one of the defined Choices
    UnknownChoice,
}

impl fmt::Display for OptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Missing => "missing",
            Self::WrongType => "wrong type",
            Self::Unresolved => "not in resolved data",
            Self::OutOfRange => "out of range",
            Self::UnknownChoice => "unknown choice",
        })
    }
}

/// A Rust type usable as a Command parameter: knows its Option Definition and how to read its
/// value from an Interaction. `Option<T>` makes any `T` not required.
///
/// `'a` is the Interaction's lifetime, so borrowed parameters (`&str`, `&User`, ...) read their
/// value without copying it.
pub trait OptionType<'a>: Sized {
    /// [Option type](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-option-type)
    /// number, e.g. 3 for string
    const OPTION_TYPE: u8;

    const REQUIRED: bool = true;

    /// Lower bound implied by the type (e.g. `0` for `u8`); explicit bounds must stay inside it
    const MIN_VALUE: Option<f64> = None;

    /// Upper bound implied by the type (e.g. `255` for `u8`); explicit bounds must stay inside it
    const MAX_VALUE: Option<f64> = None;

    fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption;

    fn extract(
        option: Option<&'a DataOption>,
        data: &'a ApplicationCommandInteractionData,
    ) -> Result<Self, OptionError>;
}

impl<'a, T: OptionType<'a>> OptionType<'a> for Option<T> {
    const OPTION_TYPE: u8 = T::OPTION_TYPE;
    const REQUIRED: bool = false;
    const MIN_VALUE: Option<f64> = T::MIN_VALUE;
    const MAX_VALUE: Option<f64> = T::MAX_VALUE;

    fn definition(meta: &OptionMeta, _required: bool) -> ApplicationCommandOption {
        T::definition(meta, false)
    }

    fn extract(
        option: Option<&'a DataOption>,
        data: &'a ApplicationCommandInteractionData,
    ) -> Result<Self, OptionError> {
        match option {
            None => Ok(None),
            Some(option) => T::extract(Some(option), data).map(Some),
        }
    }
}

fn base<const T: u8>(meta: &OptionMeta, required: bool) -> BaseOption<T> {
    BaseOption {
        t: TypeField,
        name: meta.name.to_string(),
        name_localizations: None,
        description: meta.description.to_string(),
        description_localizations: None,
        required: Some(required),
    }
}

fn string_definition(
    meta: &OptionMeta,
    required: bool,
    choices: Option<Vec<ApplicationCommandOptionChoice<String>>>,
) -> ApplicationCommandOption {
    ApplicationCommandOption::String(StringOption {
        t: TypeField,
        name: meta.name.to_string(),
        name_localizations: None,
        description: meta.description.to_string(),
        description_localizations: None,
        required: Some(required),
        choices,
        min_length: meta.min_length,
        max_length: meta.max_length,
        autocomplete: None,
    })
}

fn integer_definition(
    meta: &OptionMeta,
    required: bool,
    choices: Option<Vec<ApplicationCommandOptionChoice<i64>>>,
    type_min: Option<f64>,
    type_max: Option<f64>,
) -> ApplicationCommandOption {
    ApplicationCommandOption::Integer(IntegerOption {
        t: TypeField,
        name: meta.name.to_string(),
        name_localizations: None,
        description: meta.description.to_string(),
        description_localizations: None,
        required: Some(required),
        choices,
        min_value: meta.min_value.or(type_min).map(|v| v as i64),
        max_value: meta.max_value.or(type_max).map(|v| v as i64),
        autocomplete: None,
    })
}

/// Definition of a string Option restricted to `(name, value)` Choices; used by `#[derive(Choice)]`
pub fn string_choices(
    meta: &OptionMeta,
    required: bool,
    choices: &[(&str, &str)],
) -> ApplicationCommandOption {
    let choices = choices
        .iter()
        .map(|(name, value)| ApplicationCommandOptionChoice {
            name: name.to_string(),
            name_localizations: None,
            value: value.to_string(),
        })
        .collect();
    string_definition(meta, required, Some(choices))
}

/// Definition of an integer Option restricted to `(name, value)` Choices; used by `#[derive(Choice)]`
pub fn integer_choices(
    meta: &OptionMeta,
    required: bool,
    choices: &[(&str, i64)],
) -> ApplicationCommandOption {
    let choices = choices
        .iter()
        .map(|(name, value)| ApplicationCommandOptionChoice {
            name: name.to_string(),
            name_localizations: None,
            value: *value,
        })
        .collect();
    integer_definition(meta, required, Some(choices), None, None)
}

fn present(option: Option<&DataOption>) -> Result<&DataOption, OptionError> {
    option.ok_or(OptionError::Missing)
}

impl<'a> OptionType<'a> for &'a str {
    const OPTION_TYPE: u8 = 3;

    fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
        string_definition(meta, required, None)
    }

    fn extract(
        option: Option<&'a DataOption>,
        _data: &'a ApplicationCommandInteractionData,
    ) -> Result<Self, OptionError> {
        string_value(option)
    }
}

impl OptionType<'_> for String {
    const OPTION_TYPE: u8 = 3;

    fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
        string_definition(meta, required, None)
    }

    fn extract(
        option: Option<&DataOption>,
        _data: &ApplicationCommandInteractionData,
    ) -> Result<Self, OptionError> {
        string_value(option).map(str::to_string)
    }
}

/// Integer value of a string or integer Option, for `#[derive(Choice)]` enums
pub fn integer_value(option: Option<&DataOption>) -> Result<i64, OptionError> {
    match present(option)? {
        DataOption::Integer(o) => Ok(o.value),
        _ => Err(OptionError::WrongType),
    }
}

/// String value of a string Option, for `#[derive(Choice)]` enums
pub fn string_value(option: Option<&DataOption>) -> Result<&str, OptionError> {
    match present(option)? {
        DataOption::String(o) => Ok(&o.value),
        _ => Err(OptionError::WrongType),
    }
}

impl OptionType<'_> for i64 {
    const OPTION_TYPE: u8 = 4;

    fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
        integer_definition(meta, required, None, None, None)
    }

    fn extract(
        option: Option<&DataOption>,
        _data: &ApplicationCommandInteractionData,
    ) -> Result<Self, OptionError> {
        integer_value(option)
    }
}

macro_rules! bounded_integer {
    ($($t:ty),*) => {$(
        impl OptionType<'_> for $t {
            const OPTION_TYPE: u8 = 4;
            const MIN_VALUE: Option<f64> = Some(<$t>::MIN as f64);
            const MAX_VALUE: Option<f64> = Some(<$t>::MAX as f64);

            fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
                integer_definition(meta, required, None, Self::MIN_VALUE, Self::MAX_VALUE)
            }

            fn extract(
                option: Option<&DataOption>,
                _data: &ApplicationCommandInteractionData,
            ) -> Result<Self, OptionError> {
                <$t>::try_from(integer_value(option)?).map_err(|_| OptionError::OutOfRange)
            }
        }
    )*};
}

bounded_integer!(i8, i16, i32, u8, u16, u32);

impl OptionType<'_> for f64 {
    const OPTION_TYPE: u8 = 10;

    fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
        ApplicationCommandOption::Number(NumberOption {
            t: TypeField,
            name: meta.name.to_string(),
            name_localizations: None,
            description: meta.description.to_string(),
            description_localizations: None,
            required: Some(required),
            choices: None,
            min_value: meta.min_value,
            max_value: meta.max_value,
            autocomplete: None,
        })
    }

    fn extract(
        option: Option<&DataOption>,
        _data: &ApplicationCommandInteractionData,
    ) -> Result<Self, OptionError> {
        match present(option)? {
            DataOption::Number(o) => Ok(o.value),
            // Discord sends whole numbers without a fraction
            DataOption::Integer(o) => Ok(o.value as f64),
            _ => Err(OptionError::WrongType),
        }
    }
}

impl OptionType<'_> for bool {
    const OPTION_TYPE: u8 = 5;

    fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
        ApplicationCommandOption::Boolean(base(meta, required))
    }

    fn extract(
        option: Option<&DataOption>,
        _data: &ApplicationCommandInteractionData,
    ) -> Result<Self, OptionError> {
        match present(option)? {
            DataOption::Boolean(o) => Ok(o.value),
            _ => Err(OptionError::WrongType),
        }
    }
}

/// A resolved Option (user, channel, role, attachment) as a reference into the Interaction's
/// resolved data, and as an owned clone of it
macro_rules! resolved {
    ($($ty:ty => $variant:ident, $code:literal, $resolve:ident, $definition:path;)*) => {$(
        impl<'a> OptionType<'a> for &'a $ty {
            const OPTION_TYPE: u8 = $code;

            fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
                $definition(meta, required)
            }

            fn extract(
                option: Option<&'a DataOption>,
                data: &'a ApplicationCommandInteractionData,
            ) -> Result<Self, OptionError> {
                match present(option)? {
                    DataOption::$variant(o) => data.$resolve(&o.value).ok_or(OptionError::Unresolved),
                    _ => Err(OptionError::WrongType),
                }
            }
        }

        impl OptionType<'_> for $ty {
            const OPTION_TYPE: u8 = $code;

            fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
                $definition(meta, required)
            }

            fn extract(
                option: Option<&DataOption>,
                data: &ApplicationCommandInteractionData,
            ) -> Result<Self, OptionError> {
                <&$ty as OptionType>::extract(option, data).cloned()
            }
        }
    )*};
}

resolved! {
    User => User, 6, resolved_user, user_definition;
    PartialChannel => Channel, 7, resolved_channel, channel_definition;
    Role => Role, 8, resolved_role, role_definition;
    Attachment => Attachment, 11, resolved_attachment, attachment_definition;
}

fn user_definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
    ApplicationCommandOption::User(base(meta, required))
}

fn channel_definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
    ApplicationCommandOption::Channel(ChannelOption {
        t: TypeField,
        name: meta.name.to_string(),
        name_localizations: None,
        description: meta.description.to_string(),
        description_localizations: None,
        required: Some(required),
        channel_types: meta.channel_types.map(<[ChannelType]>::to_vec),
    })
}

fn role_definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
    ApplicationCommandOption::Role(base(meta, required))
}

fn attachment_definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
    ApplicationCommandOption::Attachment(base(meta, required))
}

/// A user together with their membership in the guild the Command was invoked in. As an Option it
/// is a user Option that fails outside guilds.
#[derive(Debug, Clone)]
pub struct ResolvedMember {
    pub user: User,
    pub member: PartialMember,
}

impl ResolvedMember {
    /// Server nickname, then global display name, then username
    pub fn display_name(&self) -> &str {
        self.member
            .nick
            .as_deref()
            .unwrap_or_else(|| self.user.display_name())
    }

    fn resolve(
        id: &crate::types::Snowflake,
        data: &ApplicationCommandInteractionData,
    ) -> Result<Self, OptionError> {
        match (data.resolved_user(id), data.resolved_member(id)) {
            (Some(user), Some(member)) => Ok(Self {
                user: user.clone(),
                member: member.clone(),
            }),
            _ => Err(OptionError::Unresolved),
        }
    }
}

impl OptionType<'_> for ResolvedMember {
    const OPTION_TYPE: u8 = 6;

    fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
        ApplicationCommandOption::User(base(meta, required))
    }

    fn extract(
        option: Option<&DataOption>,
        data: &ApplicationCommandInteractionData,
    ) -> Result<Self, OptionError> {
        match present(option)? {
            DataOption::User(o) => Self::resolve(&o.value, data),
            _ => Err(OptionError::WrongType),
        }
    }
}

/// Value of a mentionable Option
#[derive(Debug, Clone)]
pub enum UserOrRole {
    User(User),
    Role(Role),
}

impl OptionType<'_> for UserOrRole {
    const OPTION_TYPE: u8 = 9;

    fn definition(meta: &OptionMeta, required: bool) -> ApplicationCommandOption {
        ApplicationCommandOption::Mentionable(base(meta, required))
    }

    fn extract(
        option: Option<&DataOption>,
        data: &ApplicationCommandInteractionData,
    ) -> Result<Self, OptionError> {
        let DataOption::Mentionable(o) = present(option)? else {
            return Err(OptionError::WrongType);
        };
        if let Some(user) = data.resolved_user(&o.value) {
            return Ok(Self::User(user.clone()));
        }
        data.resolved_role(&o.value)
            .map(|role| Self::Role(role.clone()))
            .ok_or(OptionError::Unresolved)
    }
}

/// The target parameter of a user or message Command. `'a` is the Interaction's lifetime, so
/// `&User` and `&Message` targets read it without copying.
pub trait CommandTarget<'a>: Sized {
    /// User or message: the kind of Command this target belongs to
    const KIND: ApplicationCommandType;

    fn extract(data: &'a ApplicationCommandInteractionData) -> Result<Self, OptionError>;
}

fn target_id(
    data: &ApplicationCommandInteractionData,
) -> Result<&crate::types::Snowflake, OptionError> {
    data.target_id.as_ref().ok_or(OptionError::Missing)
}

impl<'a> CommandTarget<'a> for &'a User {
    const KIND: ApplicationCommandType = ApplicationCommandType::User;

    fn extract(data: &'a ApplicationCommandInteractionData) -> Result<Self, OptionError> {
        data.resolved_user(target_id(data)?)
            .ok_or(OptionError::Unresolved)
    }
}

impl CommandTarget<'_> for User {
    const KIND: ApplicationCommandType = ApplicationCommandType::User;

    fn extract(data: &ApplicationCommandInteractionData) -> Result<Self, OptionError> {
        <&User as CommandTarget>::extract(data).cloned()
    }
}

impl CommandTarget<'_> for ResolvedMember {
    const KIND: ApplicationCommandType = ApplicationCommandType::User;

    fn extract(data: &ApplicationCommandInteractionData) -> Result<Self, OptionError> {
        Self::resolve(target_id(data)?, data)
    }
}

impl<'a> CommandTarget<'a> for &'a Message {
    const KIND: ApplicationCommandType = ApplicationCommandType::Message;

    fn extract(data: &'a ApplicationCommandInteractionData) -> Result<Self, OptionError> {
        data.resolved_message(target_id(data)?)
            .ok_or(OptionError::Unresolved)
    }
}

impl CommandTarget<'_> for Message {
    const KIND: ApplicationCommandType = ApplicationCommandType::Message;

    fn extract(data: &ApplicationCommandInteractionData) -> Result<Self, OptionError> {
        <&Message as CommandTarget>::extract(data).cloned()
    }
}
