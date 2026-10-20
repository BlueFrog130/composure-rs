//! Modals as typed forms (`#[derive(Modal)]`): every field is a [`Label`] around one input, its
//! Rust type deciding whether the input is required and how the submitted value is read.

use crate::types::{
    Attachment, ChannelType, Checkbox, CheckboxGroup, Component, FileUpload, GroupOption,
    InteractionResponse, Label, ModalCallbackData, ModalSubmitComponent, ModalSubmitData,
    PartialChannel, RadioGroup, Role, SelectMenu, SelectOption, Snowflake, TextInput,
    TextInputStyle, User,
};

use super::HandlerError;

/// A form shown as a modal. Derived with `#[derive(Modal)]`.
pub trait Modal: Sized {
    /// Shown at the top of the modal (max 45 characters)
    const TITLE: &'static str;

    /// The modal's top-level components: text displays and one [`Label`] per field
    fn components() -> Vec<Component>;

    /// The submitted fields
    fn extract(data: &ModalSubmitData) -> Result<Self, HandlerError>;

    /// This form as a modal response with `custom_id`
    fn response(custom_id: String) -> InteractionResponse {
        InteractionResponse::Modal(ModalCallbackData {
            custom_id,
            title: Self::TITLE.to_string(),
            components: Some(Self::components()),
            tts: None,
            content: None,
            embeds: None,
            allowed_mentions: None,
            flags: None,
        })
    }
}

/// What a submitted input carries; [`ModalValue::SHAPE`] says which ones a type reads
pub mod shape {
    /// `value` text: text inputs, radio groups
    pub const TEXT: u8 = 1;
    /// `values`: selects, checkbox groups, file uploads
    pub const VALUES: u8 = 2;
    /// `value` boolean: checkboxes
    pub const CHECKED: u8 = 4;
}

/// A Modal field's Rust type: whether its input is required and how its submitted value is read.
///
/// `String` (required), `Option<String>`, `Vec<String>`, `bool` (checkbox), `Snowflake`, and the
/// resolved `User`, `Role`, `PartialChannel`, `Attachment`, plus `#[derive(Choice)]` enums, each
/// also as `Option<T>` and `Vec<T>`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be a modal field",
    note = "use String, bool, Snowflake, User, Role, PartialChannel, Attachment, or a #[derive(Choice)] enum, optionally in Option or Vec"
)]
pub trait ModalValue: Sized {
    /// Whether the input must be filled
    const REQUIRED: bool;

    /// Which submissions it reads: a combination of [`shape`] flags
    const SHAPE: u8;

    /// Several selections (a `Vec`)
    const MANY: bool = false;

    /// `component` is the submitted input, `None` when Discord left it out (nothing entered)
    fn extract(
        component: Option<&ModalSubmitComponent>,
        data: &ModalSubmitData,
    ) -> Result<Self, String>;
}

fn texts(component: Option<&ModalSubmitComponent>) -> Vec<&str> {
    match component {
        Some(component) => match (component.text(), &component.values) {
            (Some(text), _) if !text.is_empty() => vec![text],
            (_, Some(values)) => values.iter().map(String::as_str).collect(),
            _ => Vec::new(),
        },
        None => Vec::new(),
    }
}

/// One-of-each impls for a single value type, from a parser of one submitted string
macro_rules! modal_value {
    ($ty:ty, $shape:expr, |$value:ident, $data:ident| $parse:expr) => {
        impl ModalValue for $ty {
            const REQUIRED: bool = true;
            const SHAPE: u8 = $shape;

            fn extract(
                component: Option<&ModalSubmitComponent>,
                data: &ModalSubmitData,
            ) -> Result<Self, String> {
                let parse =
                    |$value: &str, $data: &ModalSubmitData| -> Result<$ty, String> { $parse };
                match texts(component).first() {
                    Some(value) => parse(value, data),
                    None => Err("missing".to_string()),
                }
            }
        }

        impl ModalValue for Option<$ty> {
            const REQUIRED: bool = false;
            const SHAPE: u8 = $shape;

            fn extract(
                component: Option<&ModalSubmitComponent>,
                data: &ModalSubmitData,
            ) -> Result<Self, String> {
                match texts(component).first() {
                    Some(_) => <$ty as ModalValue>::extract(component, data).map(Some),
                    None => Ok(None),
                }
            }
        }

        impl ModalValue for Vec<$ty> {
            const REQUIRED: bool = false;
            const SHAPE: u8 = $shape & shape::VALUES;
            const MANY: bool = true;

            fn extract(
                component: Option<&ModalSubmitComponent>,
                data: &ModalSubmitData,
            ) -> Result<Self, String> {
                let parse =
                    |$value: &str, $data: &ModalSubmitData| -> Result<$ty, String> { $parse };
                texts(component)
                    .into_iter()
                    .map(|value| parse(value, data))
                    .collect()
            }
        }
    };
}

modal_value!(String, shape::TEXT | shape::VALUES, |value, _data| Ok(
    value.to_string()
));

modal_value!(Snowflake, shape::VALUES, |value, _data| value
    .parse()
    .map_err(|_| format!("`{value}` is not an id")));

modal_value!(User, shape::VALUES, |value, data| resolve(
    value,
    data,
    ModalSubmitData::resolved_user
));

modal_value!(Role, shape::VALUES, |value, data| resolve(
    value,
    data,
    ModalSubmitData::resolved_role
));

modal_value!(PartialChannel, shape::VALUES, |value, data| resolve(
    value,
    data,
    ModalSubmitData::resolved_channel
));

modal_value!(Attachment, shape::VALUES, |value, data| resolve(
    value,
    data,
    ModalSubmitData::resolved_attachment
));

fn resolve<T: Clone>(
    value: &str,
    data: &ModalSubmitData,
    get: for<'d> fn(&'d ModalSubmitData, &Snowflake) -> Option<&'d T>,
) -> Result<T, String> {
    let id: Snowflake = value
        .parse()
        .map_err(|_| format!("`{value}` is not an id"))?;
    get(data, &id)
        .cloned()
        .ok_or_else(|| format!("{id} is not in the resolved data"))
}

impl ModalValue for bool {
    /// Checkboxes cannot be required
    const REQUIRED: bool = false;
    const SHAPE: u8 = shape::CHECKED;

    fn extract(
        component: Option<&ModalSubmitComponent>,
        _data: &ModalSubmitData,
    ) -> Result<Self, String> {
        Ok(component
            .and_then(ModalSubmitComponent::checked)
            .unwrap_or(false))
    }
}

/// The Choices of a `#[derive(Choice)]` enum, as modal options. Implemented by the derive.
pub trait ModalChoice: Sized {
    /// `(name, value)` of every variant
    const OPTIONS: &'static [(&'static str, &'static str)];

    /// The variant whose value is `value`, as submitted by an input
    fn from_modal_value(value: &str) -> Option<Self>;
}

fn choice<C: ModalChoice>(value: &str) -> Result<C, String> {
    C::from_modal_value(value).ok_or_else(|| format!("`{value}` is not a choice"))
}

impl<C: ModalChoice> ModalValue for C {
    const REQUIRED: bool = true;
    const SHAPE: u8 = shape::TEXT | shape::VALUES;

    fn extract(
        component: Option<&ModalSubmitComponent>,
        _data: &ModalSubmitData,
    ) -> Result<Self, String> {
        match texts(component).first() {
            Some(value) => choice(value),
            None => Err("missing".to_string()),
        }
    }
}

impl<C: ModalChoice> ModalValue for Option<C> {
    const REQUIRED: bool = false;
    const SHAPE: u8 = shape::TEXT | shape::VALUES;

    fn extract(
        component: Option<&ModalSubmitComponent>,
        _data: &ModalSubmitData,
    ) -> Result<Self, String> {
        texts(component)
            .first()
            .map(|value| choice(value))
            .transpose()
    }
}

impl<C: ModalChoice> ModalValue for Vec<C> {
    const REQUIRED: bool = false;
    const SHAPE: u8 = shape::VALUES;
    const MANY: bool = true;

    fn extract(
        component: Option<&ModalSubmitComponent>,
        _data: &ModalSubmitData,
    ) -> Result<Self, String> {
        texts(component).into_iter().map(choice).collect()
    }
}

/// Where a select, radio group, or checkbox group field gets its options when the field lists
/// none: from its `#[derive(Choice)]` enum
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no options to offer",
    note = "list them with `option(\"Label\", \"value\")` in the field's attribute, or use a #[derive(Choice)] enum"
)]
pub trait ModalOptions {
    /// `(label, value)` of every option
    const OPTIONS: &'static [(&'static str, &'static str)];
}

impl<C: ModalChoice> ModalOptions for C {
    const OPTIONS: &'static [(&'static str, &'static str)] = C::OPTIONS;
}

impl<C: ModalChoice> ModalOptions for Option<C> {
    const OPTIONS: &'static [(&'static str, &'static str)] = C::OPTIONS;
}

impl<C: ModalChoice> ModalOptions for Vec<C> {
    const OPTIONS: &'static [(&'static str, &'static str)] = C::OPTIONS;
}

/// Settings of one Modal field, from its attribute; builds the field's [`Label`]
#[derive(Debug, Clone, Copy)]
pub struct FieldMeta {
    /// The input's `custom_id`, which the submitted value is found by: `id`, or the field name
    pub custom_id: &'static str,
    /// Shown above the input: `label`, or the field name
    pub label: &'static str,
    /// Shown under the label
    pub description: Option<&'static str>,
    /// Shown while the input is empty (text inputs, selects)
    pub placeholder: Option<&'static str>,
    /// Fewest selections (selects, checkbox groups, file uploads)
    pub min: Option<u8>,
    /// Most selections (selects, checkbox groups, file uploads)
    pub max: Option<u8>,
    /// From the field's type
    pub required: bool,
    /// From the field's type: a `Vec`
    pub many: bool,
}

/// Text input settings, from `#[input]`
#[derive(Debug, Clone, Copy, Default)]
pub struct TextInputMeta {
    /// Multi-line instead of single-line
    pub paragraph: bool,
    /// Fewest characters, 0-4000
    pub min_length: Option<i32>,
    /// Most characters, 1-4000
    pub max_length: Option<i32>,
    /// Pre-filled text
    pub value: Option<&'static str>,
}

impl FieldMeta {
    fn label(&self, component: Component) -> Component {
        let mut label = Label::new(self.label, component);
        label.description = self.description.map(str::to_string);
        Component::Label(label)
    }

    /// Discord requires at least one selection unless `required` is off
    fn required(&self) -> Option<bool> {
        (!self.required).then_some(false)
    }

    fn max(&self, all: usize) -> Option<u8> {
        self.max
            .or_else(|| self.many.then(|| all.min(u8::MAX as usize) as u8))
    }

    /// The field as a [`TextInput`] in its [`Label`]
    pub fn text_input(&self, input: &TextInputMeta) -> Component {
        let style = if input.paragraph {
            TextInputStyle::Paragraph
        } else {
            TextInputStyle::Short
        };
        let mut text = TextInput::new(
            self.custom_id.to_string(),
            style,
            String::new(),
            input.min_length,
            input.max_length,
            self.required(),
            input.value.map(str::to_string),
            self.placeholder.map(str::to_string),
        );
        text.label = None;
        self.label(Component::TextInput(text))
    }

    /// The field as a string select of `(label, value)` options in its [`Label`]
    pub fn string_select(&self, options: &[(&str, &str)]) -> Component {
        let all = options.len();
        let options = options
            .iter()
            .map(|(label, value)| {
                SelectOption::new(label.to_string(), value.to_string(), None, None, None)
            })
            .collect();
        self.label(Component::StringSelect(self.select(all, Some(options))))
    }

    /// The field as a user select in its [`Label`]
    pub fn user_select(&self) -> Component {
        self.label(Component::UserSelect(self.select(25, None)))
    }

    /// The field as a role select in its [`Label`]
    pub fn role_select(&self) -> Component {
        self.label(Component::RoleSelect(self.select(25, None)))
    }

    /// The field as a user-and-role select in its [`Label`]
    pub fn mentionable_select(&self) -> Component {
        self.label(Component::MentionableSelect(self.select(25, None)))
    }

    /// The field as a channel select offering `channel_types` (all when `None`) in its [`Label`]
    pub fn channel_select(&self, channel_types: Option<&[ChannelType]>) -> Component {
        let mut select = self.select::<8>(25, None);
        select.channel_types = channel_types.map(<[_]>::to_vec);
        self.label(Component::ChannelSelect(select))
    }

    fn select<const T: u8>(&self, all: usize, options: Option<Vec<SelectOption>>) -> SelectMenu<T> {
        let mut select = SelectMenu::new(
            self.custom_id.to_string(),
            options,
            None,
            self.placeholder.map(str::to_string),
            self.min.map(i32::from),
            self.max(all).map(i32::from),
            None,
        );
        select.required = self.required();
        select
    }

    /// The field as a [`RadioGroup`] of `(label, value)` options in its [`Label`]
    pub fn radio(&self, options: &[(&str, &str)]) -> Component {
        let mut radio = RadioGroup::new(self.custom_id, group_options(options));
        radio.required = self.required();
        self.label(Component::RadioGroup(radio))
    }

    /// The field as a [`CheckboxGroup`] of `(label, value)` options in its [`Label`]
    pub fn checkboxes(&self, options: &[(&str, &str)]) -> Component {
        let mut group = CheckboxGroup::new(self.custom_id, group_options(options));
        group.min_values = self.min;
        group.max_values = self.max;
        group.required = self.required();
        self.label(Component::CheckboxGroup(group))
    }

    /// The field as a [`Checkbox`], checked initially when `default`, in its [`Label`].
    /// Checkboxes cannot be required.
    pub fn checkbox(&self, default: bool) -> Component {
        let mut checkbox = Checkbox::new(self.custom_id);
        checkbox.default = default.then_some(true);
        self.label(Component::Checkbox(checkbox))
    }

    /// The field as a [`FileUpload`] accepting `file_types` (any when `None`) in its [`Label`]
    pub fn file_upload(&self, file_types: Option<&[&str]>) -> Component {
        let mut upload = FileUpload::new(self.custom_id);
        upload.min_values = self.min;
        upload.max_values = self.max(10);
        upload.required = self.required();
        upload.file_types = file_types.map(|types| types.iter().map(|t| t.to_string()).collect());
        self.label(Component::FileUpload(upload))
    }

    /// Reads this field's submitted value
    pub fn extract<T: ModalValue>(&self, data: &ModalSubmitData) -> Result<T, HandlerError> {
        T::extract(data.find(self.custom_id), data)
            .map_err(|e| HandlerError::invalid_option(self.custom_id, e))
    }
}

fn group_options(options: &[(&str, &str)]) -> Vec<GroupOption> {
    options
        .iter()
        .map(|(label, value)| GroupOption::new(*label, *value))
        .collect()
}
