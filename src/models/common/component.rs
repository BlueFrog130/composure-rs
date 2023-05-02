use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::models::{ChannelType, PartialEmoji, TypeField};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Component {
    ActionRow(ActionRow),
    Button(ButtonComponent),
    StringSelect(SelectMenu<3>),
    TextInput(TextInput),
    UserSelect(SelectMenu<5>),
    RoleSelect(SelectMenu<6>),
    MentionableSelect(SelectMenu<7>),
    ChannelSelect(SelectMenu<8>),
}

impl<'de> Deserialize<'de> for Component {
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
            1 => Ok(Component::ActionRow(
                ActionRow::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            2 => Ok(Component::Button(
                ButtonComponent::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            3 => Ok(Component::StringSelect(
                SelectMenu::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            4 => Ok(Component::TextInput(
                TextInput::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            5 => Ok(Component::UserSelect(
                SelectMenu::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            6 => Ok(Component::RoleSelect(
                SelectMenu::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            7 => Ok(Component::MentionableSelect(
                SelectMenu::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            8 => Ok(Component::ChannelSelect(
                SelectMenu::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            _ => Err(serde::de::Error::custom("Unknown component")),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ActionRow {
    #[serde(rename = "type")]
    pub t: TypeField<1>,

    pub components: Vec<Component>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ButtonComponent {
    #[serde(rename = "type")]
    pub t: TypeField<2>,

    /// Button style
    pub style: ButtonStyle,

    /// Text that appears on the button; max 80 characters
    pub label: Option<String>,

    /// name, id, and animated
    pub emoji: Option<PartialEmoji>,

    /// Developer-defined identifier for the button; max 100 characters
    pub custom_id: Option<String>,

    /// URL for link-style buttons
    pub url: Option<String>,

    /// Whether the button is disabled (defaults to false)
    pub disabled: Option<bool>,
}

#[derive(Debug, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum ButtonStyle {
    /// Blurple
    Primary = 1,

    /// Grey
    Secondary = 2,

    /// Green
    Success = 3,

    /// Red
    Danger = 4,

    /// Grey, navigates to URL
    Link = 5,
}

/// [Select Menu Structure](https://discord.comundefinedhttps://discord.com/developers/docs/interactions/message-components#select-menu-object-select-menu-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct SelectMenu<const T: u8> {
    /// [Type](https://discord.com/developers/docs/interactions/message-components#component-object-component-types) of select menu component (text: 3, user: 5, role: 6, mentionable: 7, channels: 8)
    #[serde(rename = "type")]
    pub t: TypeField<T>,

    /// ID for the select menu; max 100 characters
    pub custom_id: String,

    /// Specified choices in a select menu (only required and available for string selects (type 3); max 25
    pub options: Option<Vec<SelectOption>>,

    /// List of channel types to include in the channel select component (type 8)
    pub channel_types: Option<Vec<ChannelType>>,

    /// Placeholder text if nothing is selected; max 150 characters
    pub placeholder: Option<String>,

    /// Minimum number of items that must be chosen (defaults to 1); min 0, max 25
    pub min_values: Option<i32>,

    /// Maximum number of items that can be chosen (defaults to 1); max 25
    pub max_values: Option<i32>,

    /// Whether select menu is disabled (defaults to false)
    pub disabled: Option<bool>,
}

/// [Select Option Structure](https://discord.comundefinedhttps://discord.com/developers/docs/interactions/message-components#select-menu-object-select-option-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct SelectOption {
    /// User-facing name of the option; max 100 characters
    pub label: String,

    /// Dev-defined value of the option; max 100 characters
    pub value: String,

    /// Additional description of the option; max 100 characters
    pub description: Option<String>,

    /// id, name, and animated
    pub emoji: Option<PartialEmoji>,

    /// Will show this option as selected by default
    pub default: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TextInput {
    #[serde(rename = "type")]
    pub t: TypeField<4>,

    /// Developer-defined identifier for the input; max 100 characters
    pub custom_id: String,

    /// The [Text Input Style](https://discord.com/developers/docs/interactions/message-components#text-inputs-text-input-styles)
    pub style: TextInputStyle,

    /// Label for this component; max 45 characters
    pub label: String,

    /// Minimum input length for a text input; min 0, max 4000
    pub min_length: Option<i32>,

    /// Maximum input length for a text input; min 1, max 4000
    pub max_length: Option<i32>,

    /// Whether this component is required to be filled (defaults to true)
    pub required: Option<bool>,

    /// Pre-filled value for this component; max 4000 characters
    pub value: Option<String>,

    /// Custom placeholder text if the input is empty; max 100 characters
    pub placeholder: Option<String>,
}

/// [Text Input Styles](https://discord.comundefinedhttps://discord.com/developers/docs/interactions/message-components#text-inputs-text-input-styles)
#[derive(Debug, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum TextInputStyle {
    /// Single-line input
    Short = 1,

    /// Multi-line input
    Paragraph = 2,
}
