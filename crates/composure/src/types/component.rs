use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::value::RawValue;
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

/// A [component](https://discord.com/developers/docs/components/reference). Buttons and selects
/// go in action rows (or a section's accessory); sections, text displays, thumbnails, media
/// galleries, files, separators and containers lay out
/// [Components V2](crate::types::MessageFlags::IsComponentsV2) messages; labels and their inputs
/// build modals.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Component {
    /// Type 1: a row of buttons or one select (`<Row>`)
    ActionRow(ActionRow),
    /// Type 2: a button (`<Button>`)
    Button(ButtonComponent),
    /// Type 3: a select of text options (`<StringSelect>`)
    StringSelect(StringSelect),
    /// Type 4: free text, in modals (`<TextInput>`)
    TextInput(TextInput),
    /// Type 5: a user select (`<UserSelect>`)
    UserSelect(UserSelect),
    /// Type 6: a role select (`<RoleSelect>`)
    RoleSelect(RoleSelect),
    /// Type 7: a user-or-role select (`<MentionableSelect>`)
    MentionableSelect(MentionableSelect),
    /// Type 8: a channel select (`<ChannelSelect>`)
    ChannelSelect(ChannelSelect),
    /// Type 9: text beside an accessory (`<Section>`)
    Section(Section),
    /// Type 10: markdown text (`<Text>`)
    TextDisplay(TextDisplay),
    /// Type 11: a small image, as a section's accessory (`<Thumbnail>`)
    Thumbnail(Thumbnail),
    /// Type 12: a grid of images and videos (`<Gallery>`)
    MediaGallery(MediaGallery),
    /// Type 13: an uploaded file (`<File>`)
    File(FileComponent),
    /// Type 14: space and a divider (`<Separator>`)
    Separator(Separator),
    /// Type 17: a framed group of components (`<Container>`)
    Container(Container),
    /// Type 18: a title above one modal input (`<Label>`)
    Label(Label),
    /// Type 19: file attachments, in modals (`<FileUpload>`)
    FileUpload(FileUpload),
    /// Type 21: pick one, in modals (`<RadioGroup>`)
    RadioGroup(RadioGroup),
    /// Type 22: pick several, in modals (`<CheckboxGroup>`)
    CheckboxGroup(CheckboxGroup),
    /// Type 23: one yes/no, in modals (`<Checkbox>`)
    Checkbox(Checkbox),
    /// A component type Discord added after this version, kept as received
    Unknown(Box<RawValue>),
}

impl Component {
    /// An [Action Row](https://discord.com/developers/docs/components/reference#action-row): up to
    /// 5 buttons, or one select
    pub fn row(components: Vec<Component>) -> Component {
        Component::ActionRow(ActionRow::new(components))
    }

    /// A [Container](https://discord.com/developers/docs/components/reference#container) for
    /// Components V2 messages: up to 40 action rows, text displays, sections, media galleries,
    /// separators and files, framed like an embed
    pub fn container(components: Vec<Component>) -> Component {
        Component::Container(Container::new(components))
    }

    /// A [Section](https://discord.com/developers/docs/components/reference#section): 1-3 text
    /// displays beside an accessory (a button or a [thumbnail](Component::thumbnail))
    pub fn section(components: Vec<Component>, accessory: Component) -> Component {
        Component::Section(Section::new(components, accessory))
    }

    /// A [Thumbnail](https://discord.com/developers/docs/components/reference#thumbnail): a
    /// small image from a url or `attachment://<filename>`, as a section's accessory
    pub fn thumbnail(url: impl Into<String>) -> Component {
        Component::Thumbnail(Thumbnail::new(url))
    }

    /// A [Media Gallery](https://discord.com/developers/docs/components/reference#media-gallery)
    /// of 1-10 images and videos
    pub fn gallery(items: Vec<MediaGalleryItem>) -> Component {
        Component::MediaGallery(MediaGallery::new(items))
    }

    /// A [File](https://discord.com/developers/docs/components/reference#file): a file uploaded
    /// with the message, referenced as `attachment://<filename>`
    pub fn file(url: impl Into<String>) -> Component {
        Component::File(FileComponent::new(url))
    }

    /// A [Separator](https://discord.com/developers/docs/components/reference#separator): a
    /// divider with small spacing
    pub fn separator() -> Component {
        Component::Separator(Separator::new())
    }

    /// The component's `id`; `None` when unset, and for [`Component::Unknown`] without one
    pub fn id(&self) -> Option<i32> {
        match self {
            Component::ActionRow(c) => c.id,
            Component::Button(c) => c.id,
            Component::StringSelect(c) => c.id,
            Component::TextInput(c) => c.id,
            Component::UserSelect(c) => c.id,
            Component::RoleSelect(c) => c.id,
            Component::MentionableSelect(c) => c.id,
            Component::ChannelSelect(c) => c.id,
            Component::Section(c) => c.id,
            Component::TextDisplay(c) => c.id,
            Component::Thumbnail(c) => c.id,
            Component::MediaGallery(c) => c.id,
            Component::File(c) => c.id,
            Component::Separator(c) => c.id,
            Component::Container(c) => c.id,
            Component::Label(c) => c.id,
            Component::FileUpload(c) => c.id,
            Component::RadioGroup(c) => c.id,
            Component::CheckboxGroup(c) => c.id,
            Component::Checkbox(c) => c.id,
            Component::Unknown(raw) => {
                #[derive(Deserialize)]
                struct Id {
                    id: Option<i64>,
                }
                serde_json::from_slice::<Id>(raw.get().as_bytes())
                    .ok()?
                    .id
                    .and_then(|id| i32::try_from(id).ok())
            }
        }
    }

    /// Sets the component's `id`, unique within the message; no-op for [`Component::Unknown`]
    pub fn with_id(mut self, id: i32) -> Component {
        let slot = match &mut self {
            Component::ActionRow(c) => &mut c.id,
            Component::Button(c) => &mut c.id,
            Component::StringSelect(c) => &mut c.id,
            Component::TextInput(c) => &mut c.id,
            Component::UserSelect(c) => &mut c.id,
            Component::RoleSelect(c) => &mut c.id,
            Component::MentionableSelect(c) => &mut c.id,
            Component::ChannelSelect(c) => &mut c.id,
            Component::Section(c) => &mut c.id,
            Component::TextDisplay(c) => &mut c.id,
            Component::Thumbnail(c) => &mut c.id,
            Component::MediaGallery(c) => &mut c.id,
            Component::File(c) => &mut c.id,
            Component::Separator(c) => &mut c.id,
            Component::Container(c) => &mut c.id,
            Component::Label(c) => &mut c.id,
            Component::FileUpload(c) => &mut c.id,
            Component::RadioGroup(c) => &mut c.id,
            Component::CheckboxGroup(c) => &mut c.id,
            Component::Checkbox(c) => &mut c.id,
            Component::Unknown(_) => return self,
        };
        *slot = Some(id);
        self
    }

    /// A [Label](https://discord.com/developers/docs/components/reference#label): `label` (and
    /// optionally a description) above one modal input
    pub fn label(label: impl Into<String>, component: Component) -> Component {
        Component::Label(Label::new(label, component))
    }

    /// Markdown text, in modals and Components V2 messages
    pub fn text(content: impl Into<String>) -> Component {
        Component::TextDisplay(TextDisplay::new(content))
    }

    /// A [`ButtonComponent`]: a `custom_id` button, or a link button with `url`
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

    /// A [`StringSelect`] offering `options`
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
    /// A [`TextInput`] with a (deprecated) built-in label; inside a [`Label`] use
    /// [`TextInput::new`] and clear `label`
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

    /// A [`UserSelect`]; `options` and `channel_types` do not apply
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

    /// A [`RoleSelect`]; `options` and `channel_types` do not apply
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

    /// A [`MentionableSelect`]; `options` and `channel_types` do not apply
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

    /// A [`ChannelSelect`] offering `channel_types` (all when `None`); `options` does not apply
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
        let raw = Box::<RawValue>::deserialize(deserializer)?;
        Component::from_raw(raw).map_err(serde::de::Error::custom)
    }
}

/// The `type` of a component, read before parsing the component as that type
#[derive(Deserialize)]
struct ComponentType {
    #[serde(rename = "type")]
    t: u64,
}

impl Component {
    /// Reads `type`, then parses the JSON once more as that component. Keeps the JSON as the
    /// slice it arrived in (no `serde_json::Value` tree), and parses with `from_slice` like the
    /// Interaction envelope so one copy of the parser serves both.
    fn from_raw(raw: Box<RawValue>) -> serde_json::Result<Self> {
        fn parse<T: DeserializeOwned>(raw: &RawValue) -> serde_json::Result<T> {
            serde_json::from_slice(raw.get().as_bytes())
        }

        let ComponentType { t } = parse(&raw)?;
        Ok(match t {
            1 => Component::ActionRow(parse(&raw)?),
            2 => Component::Button(parse(&raw)?),
            3 => Component::StringSelect(parse(&raw)?),
            4 => Component::TextInput(parse(&raw)?),
            5 => Component::UserSelect(parse(&raw)?),
            6 => Component::RoleSelect(parse(&raw)?),
            7 => Component::MentionableSelect(parse(&raw)?),
            8 => Component::ChannelSelect(parse(&raw)?),
            9 => Component::Section(parse(&raw)?),
            10 => Component::TextDisplay(parse(&raw)?),
            11 => Component::Thumbnail(parse(&raw)?),
            12 => Component::MediaGallery(parse(&raw)?),
            13 => Component::File(parse(&raw)?),
            14 => Component::Separator(parse(&raw)?),
            17 => Component::Container(parse(&raw)?),
            18 => Component::Label(parse(&raw)?),
            19 => Component::FileUpload(parse(&raw)?),
            21 => Component::RadioGroup(parse(&raw)?),
            22 => Component::CheckboxGroup(parse(&raw)?),
            23 => Component::Checkbox(parse(&raw)?),
            _ => Component::Unknown(raw),
        })
    }
}

/// [Action Row](https://discord.com/developers/docs/components/reference#action-row): a row of up
/// to 5 buttons, or one select. In markup: [`<Row>`](https://bluefrog130.github.io/composure-rs/views.html#row-alias-actionrow).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionRow {
    #[serde(rename = "type")]
    /// The component type, always `1`
    pub t: TypeField<1>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Up to 5 buttons, or one select
    pub components: Vec<Component>,
}

impl ActionRow {
    /// A row of `components`: up to 5 buttons, or one select
    pub fn new(components: Vec<Component>) -> Self {
        Self {
            t: TypeField,
            id: None,
            components,
        }
    }
}

/// [Button](https://discord.com/developers/docs/components/reference#button): in an
/// [`ActionRow`], or as a [`Section`]'s accessory. In markup:
/// [`<Button>`](https://bluefrog130.github.io/composure-rs/views.html#button).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ButtonComponent {
    #[serde(rename = "type")]
    /// The component type, always `2`
    pub t: TypeField<2>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Color and behavior
    pub style: ButtonStyle,

    /// Text that appears on the button; max 80 characters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// Shown before the label: a unicode emoji (`name`) or a custom one (`id`, `name`, `animated`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PartialEmoji>,

    /// Developer-defined identifier for the button; max 100 characters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<String>,

    /// URL for link-style buttons
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    /// Whether the button is disabled (defaults to false)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<bool>,

    /// Premium buttons: the SKU to buy
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sku_id: Option<Snowflake>,
}

impl ButtonComponent {
    /// A button: give it a `custom_id` (styles `Primary` to `Danger`) or a `url` (`Link`); see
    /// [`ButtonComponent::premium`] for `Premium`
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
            id: None,
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

/// [Button Style](https://discord.com/developers/docs/components/reference#button-button-styles):
/// the color and behavior of a button. In markup, the `primary`, `secondary`, `success` and
/// `danger` flags, or `style=..`.
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

/// [Select menu](https://discord.com/developers/docs/components/reference#string-select): in an
/// [`ActionRow`] in messages, or a [`Label`] in modals. `T` is the component type; use the
/// aliases [`StringSelect`], [`UserSelect`], [`RoleSelect`], [`MentionableSelect`] and
/// [`ChannelSelect`]. In markup: [`<StringSelect>`](https://bluefrog130.github.io/composure-rs/views.html#stringselect-alias-select),
/// [`<UserSelect>` and the other entity selects](https://bluefrog130.github.io/composure-rs/views.html#userselect-roleselect-mentionableselect-channelselect).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelectMenu<const T: u8> {
    /// [Type](https://discord.com/developers/docs/interactions/message-components#component-object-component-types) of select menu component (text: 3, user: 5, role: 6, mentionable: 7, channels: 8)
    #[serde(rename = "type")]
    pub t: TypeField<T>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

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
    /// A select of type `T`; `options` only apply to string selects, `channel_types` only to
    /// channel selects
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
            id: None,
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
    /// The preselected entity
    pub id: Snowflake,
    /// `"user"`, `"role"`, or `"channel"`
    #[serde(rename = "type")]
    pub kind: String,
}

impl SelectDefaultValue {
    /// A preselected user
    pub fn user(id: Snowflake) -> Self {
        Self::of(id, "user")
    }

    /// A preselected role
    pub fn role(id: Snowflake) -> Self {
        Self::of(id, "role")
    }

    /// A preselected channel
    pub fn channel(id: Snowflake) -> Self {
        Self::of(id, "channel")
    }

    fn of(id: Snowflake, kind: &str) -> Self {
        Self {
            id,
            kind: kind.to_string(),
        }
    }
}

/// [Select option](https://discord.com/developers/docs/components/reference#string-select-select-option-structure)
/// of a [`StringSelect`]. In markup: [`<Option>`](https://bluefrog130.github.io/composure-rs/views.html#option).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SelectOption {
    /// User-facing name of the option; max 100 characters
    pub label: String,

    /// Dev-defined value of the option; max 100 characters
    pub value: String,

    /// Additional description of the option; max 100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Shown before the label: a unicode emoji (`name`) or a custom one (`id`, `name`, `animated`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PartialEmoji>,

    /// Will show this option as selected by default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

impl SelectOption {
    /// An option showing `label` and submitting `value`
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

/// [Text Input](https://discord.com/developers/docs/components/reference#text-input): free text in
/// a modal, inside a [`Label`]. In markup: [`<TextInput>`](https://bluefrog130.github.io/composure-rs/views.html#textinput).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextInput {
    #[serde(rename = "type")]
    /// The component type, always `4`
    pub t: TypeField<4>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

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
    /// A text input. `label` is the deprecated built-in label: set the field to `None` when the
    /// input goes in a [`Label`], as `rsx!` and `#[derive(Modal)]` do.
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
            id: None,
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

/// [Text Display](https://discord.com/developers/docs/components/reference#text-display): markdown,
/// in Components V2 messages and modals. In markup:
/// [`<Text>`](https://bluefrog130.github.io/composure-rs/views.html#text-alias-textdisplay), or bare text.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextDisplay {
    #[serde(rename = "type")]
    /// The component type, always `10`
    pub t: TypeField<10>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Markdown: headings, lists, links, mentions, `-#` subtext
    pub content: String,
}

impl TextDisplay {
    /// Markdown text
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            t: TypeField,
            id: None,
            content: content.into(),
        }
    }
}

/// [Container](https://discord.com/developers/docs/components/reference#container): top-level
/// component of a Components V2 message, grouping others in a frame with an optional accent bar.
/// In markup: [`<Container>`](https://bluefrog130.github.io/composure-rs/views.html#container).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Container {
    #[serde(rename = "type")]
    /// The component type, always `17`
    pub t: TypeField<17>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Up to 40 action rows, text displays, sections, media galleries, separators and files
    pub components: Vec<Component>,

    /// Color of the accent bar on the left; none when unset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u32>,

    /// Blurs the content until clicked
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
}

impl Container {
    /// A container of `components` without an accent color ([`with_accent_color`](Self::with_accent_color) sets one)
    pub fn new(components: Vec<Component>) -> Self {
        Self {
            t: TypeField,
            id: None,
            components,
            accent_color: None,
            spoiler: None,
        }
    }

    /// Sets the accent bar's color (`0xRRGGBB`)
    pub fn with_accent_color(mut self, color: u32) -> Self {
        self.accent_color = Some(color);
        self
    }

    /// Blurs the content until clicked
    pub fn spoiler(mut self) -> Self {
        self.spoiler = Some(true);
        self
    }
}

/// [Section](https://discord.com/developers/docs/components/reference#section): text beside an
/// accessory, in Components V2 messages. In markup: [`<Section>`](https://bluefrog130.github.io/composure-rs/views.html#section).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Section {
    #[serde(rename = "type")]
    /// The component type, always `9`
    pub t: TypeField<9>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// 1-3 text displays
    pub components: Vec<Component>,

    /// A button or a thumbnail
    pub accessory: Box<Component>,
}

impl Section {
    /// `components` (1-3 text displays) beside `accessory` (a button or a thumbnail)
    pub fn new(components: Vec<Component>, accessory: Component) -> Self {
        Self {
            t: TypeField,
            id: None,
            components,
            accessory: Box::new(accessory),
        }
    }
}

/// A url, or an `attachment://<filename>` reference to a file uploaded with the message, that
/// Discord [unfurls](https://discord.com/developers/docs/components/reference#unfurled-media-item);
/// only `url` is sent, the rest is filled in by Discord
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnfurledMedia {
    /// The media's URL, or `attachment://<filename>` for a file uploaded with the message; the only field sent
    pub url: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Discord's cached copy of the media
    pub proxy_url: Option<String>,

    /// Images and videos only
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,

    /// Images and videos only
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,

    /// [Thumbhash](https://evanw.github.io/thumbhash/) of an image or video
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Version of the `placeholder` format
    pub placeholder_version: Option<u8>,

    /// Media type
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Whether the image is animated
    pub flags: Option<UnfurledMediaFlags>,

    /// The uploaded attachment, for `attachment://` media
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_id: Option<Snowflake>,
}

impl UnfurledMedia {
    /// Media at `url` (or `attachment://<filename>`), for Discord to unfurl
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            proxy_url: None,
            height: None,
            width: None,
            placeholder: None,
            placeholder_version: None,
            content_type: None,
            flags: None,
            attachment_id: None,
        }
    }
}

bitflags::bitflags! {
    /// [Unfurled Media Item Flags](https://discord.com/developers/docs/components/reference#unfurled-media-item-unfurled-media-item-flags)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct UnfurledMediaFlags: u32 {
        /// The image is animated
        const IsAnimated = 1 << 0;
    }
}

impl Serialize for UnfurledMediaFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> Deserialize<'de> for UnfurledMediaFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self::from_bits_retain(u32::deserialize(deserializer)?))
    }
}

/// [Thumbnail](https://discord.com/developers/docs/components/reference#thumbnail): a small image
/// as a [`Section`]'s accessory. In markup: [`<Thumbnail>`](https://bluefrog130.github.io/composure-rs/views.html#thumbnail).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Thumbnail {
    #[serde(rename = "type")]
    /// The component type, always `11`
    pub t: TypeField<11>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// An image (GIF and WebP included); no videos
    pub media: UnfurledMedia,

    /// Alt text; max 1024 characters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Blurs the image until clicked
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
}

impl Thumbnail {
    /// A thumbnail of the image at `url`, or `attachment://<filename>`
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            t: TypeField,
            id: None,
            media: UnfurledMedia::new(url),
            description: None,
            spoiler: None,
        }
    }

    /// Sets the alt text
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Blurs the image until clicked
    pub fn spoiler(mut self) -> Self {
        self.spoiler = Some(true);
        self
    }
}

/// [Media Gallery](https://discord.com/developers/docs/components/reference#media-gallery): a grid
/// of images and videos. In markup: [`<Gallery>`](https://bluefrog130.github.io/composure-rs/views.html#gallery-alias-mediagallery).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaGallery {
    #[serde(rename = "type")]
    /// The component type, always `12`
    pub t: TypeField<12>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// 1-10 items
    pub items: Vec<MediaGalleryItem>,
}

impl MediaGallery {
    /// A gallery of `items` (1-10)
    pub fn new(items: Vec<MediaGalleryItem>) -> Self {
        Self {
            t: TypeField,
            id: None,
            items,
        }
    }
}

/// [Media Gallery Item](https://discord.com/developers/docs/components/reference#media-gallery-media-gallery-item-structure):
/// one image or video of a [`MediaGallery`]. In markup:
/// [`<Item>`](https://bluefrog130.github.io/composure-rs/views.html#item-alias-mediagalleryitem).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaGalleryItem {
    /// An image or video
    pub media: UnfurledMedia,

    /// Alt text; max 1024 characters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Blurs the media until clicked
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
}

impl MediaGalleryItem {
    /// An image or video from a url or `attachment://<filename>`
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            media: UnfurledMedia::new(url),
            description: None,
            spoiler: None,
        }
    }

    /// Sets the alt text
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Blurs the media until clicked
    pub fn spoiler(mut self) -> Self {
        self.spoiler = Some(true);
        self
    }
}

/// [File](https://discord.com/developers/docs/components/reference#file): one file uploaded with
/// the message, shown inline. In markup: [`<File>`](https://bluefrog130.github.io/composure-rs/views.html#file).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileComponent {
    #[serde(rename = "type")]
    /// The component type, always `13`
    pub t: TypeField<13>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// `attachment://<filename>` only
    pub file: UnfurledMedia,

    /// Blurs the file until clicked
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,

    /// File name; filled in by Discord
    #[serde(default, skip_serializing)]
    pub name: Option<String>,

    /// Size in bytes; filled in by Discord
    #[serde(default, skip_serializing)]
    pub size: Option<u64>,
}

impl FileComponent {
    /// The uploaded file `attachment://<filename>`
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            t: TypeField,
            id: None,
            file: UnfurledMedia::new(url),
            spoiler: None,
            name: None,
            size: None,
        }
    }

    /// Blurs the file until clicked
    pub fn spoiler(mut self) -> Self {
        self.spoiler = Some(true);
        self
    }
}

/// [Separator](https://discord.com/developers/docs/components/reference#separator): vertical
/// space between components, with a divider line by default. In markup:
/// [`<Separator>`](https://bluefrog130.github.io/composure-rs/views.html#separator).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Separator {
    #[serde(rename = "type")]
    /// The component type, always `14`
    pub t: TypeField<14>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Draws a line (defaults to true)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub divider: Option<bool>,

    /// Defaults to small
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spacing: Option<SeparatorSpacing>,
}

impl Separator {
    /// A divider with small spacing
    pub fn new() -> Self {
        Self {
            t: TypeField,
            id: None,
            divider: None,
            spacing: None,
        }
    }

    /// Only space, no line
    pub fn without_divider(mut self) -> Self {
        self.divider = Some(false);
        self
    }

    /// Sets the space around it
    pub fn with_spacing(mut self, spacing: SeparatorSpacing) -> Self {
        self.spacing = Some(spacing);
        self
    }
}

impl Default for Separator {
    fn default() -> Self {
        Self::new()
    }
}

/// Space around a [`Separator`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum SeparatorSpacing {
    /// Small space (the default)
    Small = 1,
    /// Large space
    Large = 2,
}

macro_rules! into_component {
    ($($variant:ident($ty:ty)),* $(,)?) => {
        $(impl From<$ty> for Component {
            fn from(component: $ty) -> Self {
                Component::$variant(component)
            }
        })*
    };
}

into_component! {
    ActionRow(ActionRow),
    Button(ButtonComponent),
    StringSelect(StringSelect),
    TextInput(TextInput),
    UserSelect(UserSelect),
    RoleSelect(RoleSelect),
    MentionableSelect(MentionableSelect),
    ChannelSelect(ChannelSelect),
    Section(Section),
    TextDisplay(TextDisplay),
    Thumbnail(Thumbnail),
    MediaGallery(MediaGallery),
    File(FileComponent),
    Separator(Separator),
    Container(Container),
    Label(Label),
    FileUpload(FileUpload),
    RadioGroup(RadioGroup),
    CheckboxGroup(CheckboxGroup),
    Checkbox(Checkbox),
}

/// [Label](https://discord.com/developers/docs/components/reference#label): a title (and
/// description) above one modal input. In markup: [`<Label>`](https://bluefrog130.github.io/composure-rs/views.html#label).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    #[serde(rename = "type")]
    /// The component type, always `18`
    pub t: TypeField<18>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Max 45 characters
    pub label: String,

    /// Max 100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A text input, select, file upload, radio group, checkbox group, or checkbox
    pub component: Box<Component>,
}

impl Label {
    /// `label` above `component`
    pub fn new(label: impl Into<String>, component: Component) -> Self {
        Self {
            t: TypeField,
            id: None,
            label: label.into(),
            description: None,
            component: Box::new(component),
        }
    }

    /// Sets the line shown under the label
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// [File Upload](https://discord.com/developers/docs/components/reference#file-upload) in a modal;
/// the submit carries attachment ids, resolved in `resolved.attachments`. In markup:
/// [`<FileUpload>`](https://bluefrog130.github.io/composure-rs/views.html#fileupload).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileUpload {
    #[serde(rename = "type")]
    /// The component type, always `19`
    pub t: TypeField<19>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Identifies the input in the submit; max 100 characters
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
    /// An upload of one file of any type, submitted under `custom_id`
    pub fn new(custom_id: impl Into<String>) -> Self {
        Self {
            t: TypeField,
            id: None,
            custom_id: custom_id.into(),
            min_values: None,
            max_values: None,
            required: None,
            file_types: None,
        }
    }
}

/// One choice of a [`RadioGroup`] or [`CheckboxGroup`]. In markup:
/// [`<Option>`](https://bluefrog130.github.io/composure-rs/views.html#option).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GroupOption {
    /// Max 100 characters
    pub value: String,

    /// Max 100 characters
    pub label: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    /// A second line under the label; max 100 characters
    pub description: Option<String>,

    /// Selected initially
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

impl GroupOption {
    /// An option showing `label` and submitting `value`
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
/// of 2-10 options; the submit carries the value (or null). In markup:
/// [`<RadioGroup>`](https://bluefrog130.github.io/composure-rs/views.html#radiogroup).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RadioGroup {
    #[serde(rename = "type")]
    /// The component type, always `21`
    pub t: TypeField<21>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Identifies the input in the submit; max 100 characters
    pub custom_id: String,

    /// 2-10 options
    pub options: Vec<GroupOption>,

    /// Defaults to true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

impl RadioGroup {
    /// A radio group of `options`, submitted under `custom_id`
    pub fn new(custom_id: impl Into<String>, options: Vec<GroupOption>) -> Self {
        Self {
            t: TypeField,
            id: None,
            custom_id: custom_id.into(),
            options,
            required: None,
        }
    }
}

/// [Checkbox Group](https://discord.com/developers/docs/components/reference#checkbox-group): pick
/// any of 1-10 options; the submit carries the selected values. In markup:
/// [`<CheckboxGroup>`](https://bluefrog130.github.io/composure-rs/views.html#checkboxgroup).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CheckboxGroup {
    #[serde(rename = "type")]
    /// The component type, always `22`
    pub t: TypeField<22>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Identifies the input in the submit; max 100 characters
    pub custom_id: String,

    /// 1-10 options
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
    /// A checkbox group of `options`, submitted under `custom_id`
    pub fn new(custom_id: impl Into<String>, options: Vec<GroupOption>) -> Self {
        Self {
            t: TypeField,
            id: None,
            custom_id: custom_id.into(),
            options,
            min_values: None,
            max_values: None,
            required: None,
        }
    }
}

/// [Checkbox](https://discord.com/developers/docs/components/reference#checkbox): a single
/// yes/no; cannot be required. The submit carries a boolean. In markup:
/// [`<Checkbox>`](https://bluefrog130.github.io/composure-rs/views.html#checkbox).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Checkbox {
    #[serde(rename = "type")]
    /// The component type, always `23`
    pub t: TypeField<23>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Identifies the input in the submit; max 100 characters
    pub custom_id: String,

    /// Checked initially
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

impl Checkbox {
    /// An unchecked checkbox submitted under `custom_id`
    pub fn new(custom_id: impl Into<String>) -> Self {
        Self {
            t: TypeField,
            id: None,
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn layout_builders_serialize_like_the_docs() {
        let message = vec![
            Component::container(vec![
                Component::section(
                    vec![Component::text("# Title"), Component::text("body")],
                    Thumbnail::new("https://example.com/a.png")
                        .description("alt")
                        .into(),
                ),
                Separator::new()
                    .without_divider()
                    .with_spacing(SeparatorSpacing::Large)
                    .into(),
                Component::gallery(vec![MediaGalleryItem::new("attachment://b.png").spoiler()]),
                Component::file("attachment://c.txt"),
                Component::row(vec![Component::new_button(
                    ButtonStyle::Primary,
                    Some("Go".into()),
                    None,
                    Some("go".into()),
                    None,
                    None,
                )]),
            ])
            .with_id(7),
            Component::separator(),
        ];
        assert_eq!(
            serde_json::to_value(&message).unwrap(),
            json!([
                { "type": 17, "id": 7, "components": [
                    { "type": 9,
                      "components": [
                        { "type": 10, "content": "# Title" },
                        { "type": 10, "content": "body" },
                      ],
                      "accessory": {
                        "type": 11,
                        "media": { "url": "https://example.com/a.png" },
                        "description": "alt",
                      } },
                    { "type": 14, "divider": false, "spacing": 2 },
                    { "type": 12, "items": [
                        { "media": { "url": "attachment://b.png" }, "spoiler": true },
                    ] },
                    { "type": 13, "file": { "url": "attachment://c.txt" } },
                    { "type": 1, "components": [
                        { "type": 2, "style": 1, "label": "Go", "custom_id": "go" },
                    ] },
                ] },
                { "type": 14 },
            ])
        );
    }

    #[test]
    fn received_components_v2_message_deserializes() {
        let media = json!({
            "id": "1", "url": "https://cdn.discordapp.com/a.png",
            "proxy_url": "https://media.discordapp.net/a.png",
            "width": 64, "height": 32, "content_type": "image/png",
            "placeholder": "abc", "placeholder_version": 1, "flags": 1,
            "attachment_id": "2", "loading_state": 2,
        });
        let json = json!([{
            "type": 17, "id": 1, "accent_color": null, "spoiler": false,
            "components": [
                { "type": 9, "id": 2,
                  "components": [{ "type": 10, "id": 3, "content": "hi" }],
                  "accessory": { "type": 11, "id": 4, "media": media,
                                 "description": null, "spoiler": false } },
                { "type": 14, "id": 5, "divider": true, "spacing": 1 },
                { "type": 12, "id": 6, "items": [
                    { "media": media, "description": null, "spoiler": false },
                ] },
                { "type": 13, "id": 7, "file": media, "name": "a.png", "size": 10,
                  "spoiler": false },
                { "type": 99, "id": 8, "new": true },
            ],
        }]);
        let components: Vec<Component> = serde_json::from_value(json).unwrap();
        let Component::Container(container) = &components[0] else {
            panic!("not a container");
        };
        assert_eq!(container.accent_color, None);
        let ids: Vec<_> = container.components.iter().map(Component::id).collect();
        assert_eq!(ids, [Some(2), Some(5), Some(6), Some(7), Some(8)]);

        let Component::Section(section) = &container.components[0] else {
            panic!("not a section");
        };
        let Component::Thumbnail(thumbnail) = section.accessory.as_ref() else {
            panic!("not a thumbnail");
        };
        assert_eq!(thumbnail.media.width, Some(64));
        assert_eq!(thumbnail.media.flags, Some(UnfurledMediaFlags::IsAnimated));
        let Component::File(file) = &container.components[3] else {
            panic!("not a file");
        };
        assert_eq!((file.name.as_deref(), file.size), (Some("a.png"), Some(10)));
        assert!(matches!(container.components[4], Component::Unknown(_)));
    }
}
