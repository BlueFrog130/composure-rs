use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::{ChannelType, PartialEmoji, Snowflake, TypeField};

/// Select menu for picking from defined text options
pub type StringSelect = SelectMenu<3>;

/// Select menu for picking from users
pub type UserSelect = SelectMenu<5>;

/// Select menu for picking from roles
pub type RoleSelect = SelectMenu<6>;

/// Select menu for picking from users and roles
pub type MentionableSelect = SelectMenu<7>;

/// Select menu for picking from channels
pub type ChannelSelect = SelectMenu<8>;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Component {
    ActionRow(ActionRow),
    Button(ButtonComponent),
    StringSelect(StringSelect),
    TextInput(TextInput),
    UserSelect(UserSelect),
    RoleSelect(RoleSelect),
    MentionableSelect(MentionableSelect),
    ChannelSelect(ChannelSelect),
    TextDisplay(TextDisplay),
    Label(Label),
    FileUpload(FileUpload),
    RadioGroup(RadioGroup),
    CheckboxGroup(CheckboxGroup),
    Checkbox(Checkbox),
}

impl Component {
    /// A [Label](https://discord.com/developers/docs/components/reference#label): `label` (and
    /// optionally a description) above one modal input
    pub fn label(label: impl Into<String>, component: Component) -> Component {
        Component::Label(Label::new(label, component))
    }

    /// Markdown text, in modals and Components V2 messages
    pub fn text(content: impl Into<String>) -> Component {
        Component::TextDisplay(TextDisplay::new(content))
    }

    pub fn new_button(
        style: ButtonStyle,
        label: Option<String>,
        emoji: Option<PartialEmoji>,
        custom_id: Option<String>,
        url: Option<String>,
        disabled: Option<bool>,
    ) -> Component {
        Self::Button(ButtonComponent::new(
            style, label, emoji, custom_id, url, disabled,
        ))
    }

    pub fn new_string_select(
        custom_id: String,
        options: Option<Vec<SelectOption>>,
        channel_types: Option<Vec<ChannelType>>,
        placeholder: Option<String>,
        min_values: Option<i32>,
        max_values: Option<i32>,
        disabled: Option<bool>,
    ) -> Component {
        Self::StringSelect(SelectMenu::new(
            custom_id,
            options,
            channel_types,
            placeholder,
            min_values,
            max_values,
            disabled,
        ))
    }

    // Mirrors the Discord text input fields; a builder would be a breaking change.
    #[allow(clippy::too_many_arguments)]
    pub fn new_text_input(
        custom_id: String,
        style: TextInputStyle,
        label: String,
        min_length: Option<i32>,
        max_length: Option<i32>,
        required: Option<bool>,
        value: Option<String>,
        placeholder: Option<String>,
    ) -> Component {
        Self::TextInput(TextInput::new(
            custom_id,
            style,
            label,
            min_length,
            max_length,
            required,
            value,
            placeholder,
        ))
    }

    pub fn new_user_select(
        custom_id: String,
        options: Option<Vec<SelectOption>>,
        channel_types: Option<Vec<ChannelType>>,
        placeholder: Option<String>,
        min_values: Option<i32>,
        max_values: Option<i32>,
        disabled: Option<bool>,
    ) -> Component {
        Self::UserSelect(SelectMenu::new(
            custom_id,
            options,
            channel_types,
            placeholder,
            min_values,
            max_values,
            disabled,
        ))
    }

    pub fn new_role_select(
        custom_id: String,
        options: Option<Vec<SelectOption>>,
        channel_types: Option<Vec<ChannelType>>,
        placeholder: Option<String>,
        min_values: Option<i32>,
        max_values: Option<i32>,
        disabled: Option<bool>,
    ) -> Component {
        Self::RoleSelect(SelectMenu::new(
            custom_id,
            options,
            channel_types,
            placeholder,
            min_values,
            max_values,
            disabled,
        ))
    }

    pub fn new_mentionable_select(
        custom_id: String,
        options: Option<Vec<SelectOption>>,
        channel_types: Option<Vec<ChannelType>>,
        placeholder: Option<String>,
        min_values: Option<i32>,
        max_values: Option<i32>,
        disabled: Option<bool>,
    ) -> Component {
        Self::MentionableSelect(SelectMenu::new(
            custom_id,
            options,
            channel_types,
            placeholder,
            min_values,
            max_values,
            disabled,
        ))
    }

    pub fn new_channel_select(
        custom_id: String,
        options: Option<Vec<SelectOption>>,
        channel_types: Option<Vec<ChannelType>>,
        placeholder: Option<String>,
        min_values: Option<i32>,
        max_values: Option<i32>,
        disabled: Option<bool>,
    ) -> Component {
        Self::ChannelSelect(SelectMenu::new(
            custom_id,
            options,
            channel_types,
            placeholder,
            min_values,
            max_values,
            disabled,
        ))
    }
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
            10 => Ok(Component::TextDisplay(
                TextDisplay::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            18 => Ok(Component::Label(
                Label::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            19 => Ok(Component::FileUpload(
                FileUpload::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            21 => Ok(Component::RadioGroup(
                RadioGroup::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            22 => Ok(Component::CheckboxGroup(
                CheckboxGroup::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            23 => Ok(Component::Checkbox(
                Checkbox::deserialize(value).map_err(serde::de::Error::custom)?,
            )),
            _ => Err(serde::de::Error::custom("Unknown component")),
        }
    }
}

/// Container for other components
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionRow {
    #[serde(rename = "type")]
    pub t: TypeField<1>,

    pub components: Vec<Component>,
}

impl ActionRow {
    pub fn new(components: Vec<Component>) -> Self {
        Self {
            t: TypeField,
            components,
        }
    }
}

/// Button Object
#[derive(Debug, Clone, Deserialize, Serialize)]
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

    /// Premium buttons: the SKU to buy
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku_id: Option<Snowflake>,
}

impl ButtonComponent {
    pub fn new(
        style: ButtonStyle,
        label: Option<String>,
        emoji: Option<PartialEmoji>,
        custom_id: Option<String>,
        url: Option<String>,
        disabled: Option<bool>,
    ) -> Self {
        Self {
            t: TypeField,
            style,
            label,
            emoji,
            custom_id,
            url,
            disabled,
            sku_id: None,
        }
    }

    /// A [premium button](https://discord.com/developers/docs/monetization/implementing-app-subscriptions#prompting-users-to-subscribe)
    /// offering `sku_id`
    pub fn premium(sku_id: Snowflake) -> Self {
        Self {
            sku_id: Some(sku_id),
            ..Self::new(ButtonStyle::Premium, None, None, None, None, None)
        }
    }
}

#[derive(Debug, Clone, Deserialize_repr, Serialize_repr)]
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

    /// Shows a premium SKU to buy; no custom_id, label, url, or emoji
    Premium = 6,
}

/// [Select Menu Structure](https://discord.com/developers/docs/interactions/message-components#select-menu-object-select-menu-structure)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelectMenu<const T: u8> {
    /// [Type](https://discord.com/developers/docs/interactions/message-components#component-object-component-types) of select menu component (text: 3, user: 5, role: 6, mentionable: 7, channels: 8)
    #[serde(rename = "type")]
    pub t: TypeField<T>,

    /// ID for the select menu; max 100 characters
    pub custom_id: String,

    /// Specified choices in a select menu (only required and available for string selects (type 3); max 25
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<SelectOption>>,

    /// List of channel types to include in the channel select component (type 8)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_types: Option<Vec<ChannelType>>,

    /// Placeholder text if nothing is selected; max 150 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    /// Minimum number of items that must be chosen (defaults to 1); min 0, max 25
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_values: Option<i32>,

    /// Maximum number of items that can be chosen (defaults to 1); max 25
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_values: Option<i32>,

    /// Whether select menu is disabled (defaults to false); not allowed in modals
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,

    /// In modals: whether a selection is required (defaults to true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// User, role, mentionable, and channel selects: preselected entities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_values: Option<Vec<SelectDefaultValue>>,
}

impl<const T: u8> SelectMenu<T> {
    pub fn new(
        custom_id: String,
        options: Option<Vec<SelectOption>>,
        channel_types: Option<Vec<ChannelType>>,
        placeholder: Option<String>,
        min_values: Option<i32>,
        max_values: Option<i32>,
        disabled: Option<bool>,
    ) -> Self {
        Self {
            t: TypeField,
            custom_id,
            options,
            channel_types,
            placeholder,
            min_values,
            max_values,
            disabled,
            required: None,
            default_values: None,
        }
    }
}

/// A [default value](https://discord.com/developers/docs/components/reference#user-select-select-default-value-structure)
/// of an entity select
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelectDefaultValue {
    pub id: Snowflake,
    /// `"user"`, `"role"`, or `"channel"`
    #[serde(rename = "type")]
    pub kind: String,
}

/// [Select Option Structure](https://discord.com/developers/docs/interactions/message-components#select-menu-object-select-option-structure)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelectOption {
    /// User-facing name of the option; max 100 characters
    pub label: String,

    /// Dev-defined value of the option; max 100 characters
    pub value: String,

    /// Additional description of the option; max 100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// id, name, and animated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PartialEmoji>,

    /// Will show this option as selected by default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

impl SelectOption {
    pub fn new(
        label: String,
        value: String,
        description: Option<String>,
        emoji: Option<PartialEmoji>,
        default: Option<bool>,
    ) -> Self {
        Self {
            label,
            value,
            description,
            emoji,
            default,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextInput {
    #[serde(rename = "type")]
    pub t: TypeField<4>,

    /// Developer-defined identifier for the input; max 100 characters
    pub custom_id: String,

    /// The [Text Input Style](https://discord.com/developers/docs/interactions/message-components#text-inputs-text-input-styles)
    pub style: TextInputStyle,

    /// Label for this component (max 45 characters); deprecated, and left out inside a
    /// [`Label`]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Minimum input length for a text input; min 0, max 4000
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<i32>,

    /// Maximum input length for a text input; min 1, max 4000
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<i32>,

    /// Whether this component is required to be filled (defaults to true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Pre-filled value for this component; max 4000 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Custom placeholder text if the input is empty; max 100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
}

impl TextInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        custom_id: String,
        style: TextInputStyle,
        label: String,
        min_length: Option<i32>,
        max_length: Option<i32>,
        required: Option<bool>,
        value: Option<String>,
        placeholder: Option<String>,
    ) -> Self {
        Self {
            t: TypeField,
            custom_id,
            style,
            label: Some(label),
            min_length,
            max_length,
            required,
            value,
            placeholder,
        }
    }
}

/// [Text Display](https://discord.com/developers/docs/components/reference#text-display): markdown
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextDisplay {
    #[serde(rename = "type")]
    pub t: TypeField<10>,

    pub content: String,
}

impl TextDisplay {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            t: TypeField,
            content: content.into(),
        }
    }
}

/// [Label](https://discord.com/developers/docs/components/reference#label): a title (and
/// description) above one modal input
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    #[serde(rename = "type")]
    pub t: TypeField<18>,

    /// Max 45 characters
    pub label: String,

    /// Max 100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A text input, select, file upload, radio group, checkbox group, or checkbox
    pub component: Box<Component>,
}

impl Label {
    pub fn new(label: impl Into<String>, component: Component) -> Self {
        Self {
            t: TypeField,
            label: label.into(),
            description: None,
            component: Box::new(component),
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// [File Upload](https://discord.com/developers/docs/components/reference#file-upload) in a modal;
/// the submit carries attachment ids, resolved in `resolved.attachments`
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileUpload {
    #[serde(rename = "type")]
    pub t: TypeField<19>,

    pub custom_id: String,

    /// 0-10, default 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_values: Option<u8>,

    /// 1-10, default 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_values: Option<u8>,

    /// Defaults to true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Up to 10 of `image`, `video`, `audio`, or `.ext`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_types: Option<Vec<String>>,
}

impl FileUpload {
    pub fn new(custom_id: impl Into<String>) -> Self {
        Self {
            t: TypeField,
            custom_id: custom_id.into(),
            min_values: None,
            max_values: None,
            required: None,
            file_types: None,
        }
    }
}

/// One choice of a [`RadioGroup`] or [`CheckboxGroup`]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GroupOption {
    /// Max 100 characters
    pub value: String,

    /// Max 100 characters
    pub label: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Selected initially
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

impl GroupOption {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            description: None,
            default: None,
        }
    }
}

/// [Radio Group](https://discord.com/developers/docs/components/reference#radio-group): pick one
/// of 2-10 options; the submit carries the value (or null)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RadioGroup {
    #[serde(rename = "type")]
    pub t: TypeField<21>,

    pub custom_id: String,

    pub options: Vec<GroupOption>,

    /// Defaults to true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

impl RadioGroup {
    pub fn new(custom_id: impl Into<String>, options: Vec<GroupOption>) -> Self {
        Self {
            t: TypeField,
            custom_id: custom_id.into(),
            options,
            required: None,
        }
    }
}

/// [Checkbox Group](https://discord.com/developers/docs/components/reference#checkbox-group): pick
/// any of 1-10 options; the submit carries the selected values
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckboxGroup {
    #[serde(rename = "type")]
    pub t: TypeField<22>,

    pub custom_id: String,

    pub options: Vec<GroupOption>,

    /// 0-10, default 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_values: Option<u8>,

    /// 1-10, default the number of options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_values: Option<u8>,

    /// Defaults to true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

impl CheckboxGroup {
    pub fn new(custom_id: impl Into<String>, options: Vec<GroupOption>) -> Self {
        Self {
            t: TypeField,
            custom_id: custom_id.into(),
            options,
            min_values: None,
            max_values: None,
            required: None,
        }
    }
}

/// [Checkbox](https://discord.com/developers/docs/components/reference#checkbox): a single
/// yes/no; cannot be required. The submit carries a boolean.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Checkbox {
    #[serde(rename = "type")]
    pub t: TypeField<23>,

    pub custom_id: String,

    /// Checked initially
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

impl Checkbox {
    pub fn new(custom_id: impl Into<String>) -> Self {
        Self {
            t: TypeField,
            custom_id: custom_id.into(),
            default: None,
        }
    }
}

/// [Text Input Styles](https://discord.com/developers/docs/interactions/message-components#text-inputs-text-input-styles)
#[derive(Debug, Clone, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum TextInputStyle {
    /// Single-line input
    Short = 1,

    /// Multi-line input
    Paragraph = 2,
}
