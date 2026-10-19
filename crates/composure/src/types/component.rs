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
/// to 5 buttons, or one select
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActionRow {
    #[serde(rename = "type")]
    pub t: TypeField<1>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    pub components: Vec<Component>,
}

impl ActionRow {
    pub fn new(components: Vec<Component>) -> Self {
        Self {
            t: TypeField,
            id: None,
            components,
        }
    }
}

/// Button Object
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ButtonComponent {
    #[serde(rename = "type")]
    pub t: TypeField<2>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Button style
    pub style: ButtonStyle,

    /// Text that appears on the button; max 80 characters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    /// name, id, and animated
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
    pub id: Snowflake,
    /// `"user"`, `"role"`, or `"channel"`
    #[serde(rename = "type")]
    pub kind: String,
}

impl SelectDefaultValue {
    pub fn user(id: Snowflake) -> Self {
        Self::of(id, "user")
    }

    pub fn role(id: Snowflake) -> Self {
        Self::of(id, "role")
    }

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

/// [Text Display](https://discord.com/developers/docs/components/reference#text-display): markdown
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TextDisplay {
    #[serde(rename = "type")]
    pub t: TypeField<10>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    pub content: String,
}

impl TextDisplay {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            t: TypeField,
            id: None,
            content: content.into(),
        }
    }
}

/// [Container](https://discord.com/developers/docs/components/reference#container): top-level
/// component of a Components V2 message, grouping others in a frame with an optional accent bar
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Container {
    #[serde(rename = "type")]
    pub t: TypeField<17>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// Up to 40 action rows, text displays, sections, media galleries, separators and files
    pub components: Vec<Component>,

    /// Color of the accent bar on the left; filled in by the App's
    /// [`Theme`](crate::command::Theme) when unset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u32>,

    /// Blurs the content until clicked
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
}

impl Container {
    pub fn new(components: Vec<Component>) -> Self {
        Self {
            t: TypeField,
            id: None,
            components,
            accent_color: None,
            spoiler: None,
        }
    }

    pub fn with_accent_color(mut self, color: u32) -> Self {
        self.accent_color = Some(color);
        self
    }

    pub fn spoiler(mut self) -> Self {
        self.spoiler = Some(true);
        self
    }
}

/// [Section](https://discord.com/developers/docs/components/reference#section): text beside an
/// accessory, in Components V2 messages
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Section {
    #[serde(rename = "type")]
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
    pub url: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    pub placeholder_version: Option<u8>,

    /// Media type
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<UnfurledMediaFlags>,

    /// The uploaded attachment, for `attachment://` media
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_id: Option<Snowflake>,
}

impl UnfurledMedia {
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
/// as a [`Section`]'s accessory
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Thumbnail {
    #[serde(rename = "type")]
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
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            t: TypeField,
            id: None,
            media: UnfurledMedia::new(url),
            description: None,
            spoiler: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn spoiler(mut self) -> Self {
        self.spoiler = Some(true);
        self
    }
}

/// [Media Gallery](https://discord.com/developers/docs/components/reference#media-gallery): a grid
/// of images and videos
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaGallery {
    #[serde(rename = "type")]
    pub t: TypeField<12>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    /// 1-10 items
    pub items: Vec<MediaGalleryItem>,
}

impl MediaGallery {
    pub fn new(items: Vec<MediaGalleryItem>) -> Self {
        Self {
            t: TypeField,
            id: None,
            items,
        }
    }
}

/// [Media Gallery Item](https://discord.com/developers/docs/components/reference#media-gallery-media-gallery-item-structure)
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

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn spoiler(mut self) -> Self {
        self.spoiler = Some(true);
        self
    }
}

/// [File](https://discord.com/developers/docs/components/reference#file): one file uploaded with
/// the message, shown inline
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileComponent {
    #[serde(rename = "type")]
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

    pub fn spoiler(mut self) -> Self {
        self.spoiler = Some(true);
        self
    }
}

/// [Separator](https://discord.com/developers/docs/components/reference#separator): vertical
/// space between components, with a divider line by default
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Separator {
    #[serde(rename = "type")]
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
    Small = 1,
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
/// description) above one modal input
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Label {
    #[serde(rename = "type")]
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
    pub fn new(label: impl Into<String>, component: Component) -> Self {
        Self {
            t: TypeField,
            id: None,
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

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

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
            id: None,
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

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

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
            id: None,
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

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

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
/// yes/no; cannot be required. The submit carries a boolean.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Checkbox {
    #[serde(rename = "type")]
    pub t: TypeField<23>,

    /// Identifier unique within the message; Discord numbers components without one
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,

    pub custom_id: String,

    /// Checked initially
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

impl Checkbox {
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
