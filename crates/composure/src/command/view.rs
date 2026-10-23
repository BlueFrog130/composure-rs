//! Views (`#[view]` and `rsx!`): message and modal layouts written as markup, rendered from
//! Props, with inline handlers for their components (ADR 0011).
//!
//! ```ignore
//! #[view]
//! pub fn Card(title: String, likes: u32) -> View {
//!     rsx! {
//!         <Container>
//!             "## " {title.as_str()}
//!             <Row>
//!                 <Button primary on:click:like=move |_| Card { likes: likes + 1, title }>
//!                     "Like (" {likes} ")"
//!                 </Button>
//!             </Row>
//!         </Container>
//!     }
//! }
//! ```

use std::{
    cell::Cell,
    fmt::{self, Display, Write},
};

use crate::types::{
    AllowedMentions, Attachment, Component, File, InteractionResponse, MessageCallbackData,
    MessageComponentData, MessageFlags, ModalCallbackData, PartialChannel, PartialEmoji, Role,
    Snowflake, User,
};

use super::{
    CommandOutput, ComponentCtx, Extras, HandlerError, IntoResponse, ModalChoice, ModalCtx, Update,
    custom_id::CustomIdError, limits,
};

/// A rendered message: Components V2 components plus the rest of the message (ephemeral,
/// allowed mentions, files to upload), answered as a new message (or, from an inline handler, as
/// an update of the message the component is on). Built by [`rsx!`](crate::rsx); see
/// [Views](https://bluefrog130.github.io/composure-rs/views.html).
///
/// A `custom_id` that could not be built (state past 100 characters) is kept inside, and
/// Discord's [limits](crate::types::limits) are checked when the View is answered with or sent:
/// either surfaces as a [`ViewError`] (a [`HandlerError`] in handlers).
#[derive(Debug, Clone)]
#[must_use]
pub struct View {
    pub(crate) components: Vec<Component>,
    pub(crate) extras: Extras,
}

impl Default for View {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl View {
    /// A View of `components`, as built by hand
    pub fn new(components: Vec<Component>) -> Self {
        Self {
            components,
            extras: Extras::default(),
        }
    }

    /// The top-level components
    pub fn components(&self) -> &[Component] {
        &self.components
    }

    /// The files uploaded with the message: added with [`file`](Self::file), or by `<File>`,
    /// `<Thumbnail>` and `<Item>` given a [`File`]
    pub fn files(&self) -> &[File] {
        self.extras.files()
    }

    /// The message flags set with [`ephemeral`](Self::ephemeral) and
    /// [`suppress_notifications`](Self::suppress_notifications); `IS_COMPONENTS_V2` is added
    /// when the message is sent
    pub fn flags(&self) -> MessageFlags {
        self.extras.flags()
    }

    /// Only the invoking user sees the message. Ignored when it updates a message, whose
    /// ephemerality cannot change.
    pub fn ephemeral(mut self) -> Self {
        self.extras.inner().flags |= MessageFlags::Ephemeral;
        self
    }

    /// The message does not trigger push and desktop notifications
    pub fn suppress_notifications(mut self) -> Self {
        self.extras.inner().flags |= MessageFlags::SuppressNotifications;
        self
    }

    /// Who the message may ping
    pub fn allowed_mentions(mut self, allowed_mentions: AllowedMentions) -> Self {
        self.extras.inner().allowed_mentions = Some(allowed_mentions);
        self
    }

    /// Uploads `file` with the message; show it with `<File url="attachment://<name>">`, or give
    /// the `File` to `<File>` directly
    pub fn file(mut self, file: File) -> Self {
        self.extras.inner().files.push(file);
        self
    }

    #[cfg_attr(not(feature = "rest"), allow(dead_code))]
    pub(crate) fn mentions(&self) -> Option<&AllowedMentions> {
        self.extras.mentions()
    }

    /// The flags a REST body sends: `IS_COMPONENTS_V2`, plus ephemeral where the Endpoint takes
    /// it
    #[cfg_attr(not(feature = "rest"), allow(dead_code))]
    pub(crate) fn message_flags(&self, ephemeral: bool) -> MessageFlags {
        let flags = MessageFlags::IsComponentsV2 | self.extras.flags();
        if ephemeral {
            flags
        } else {
            flags - MessageFlags::Ephemeral
        }
    }

    /// Whether the View can be sent: its `custom_id`s were built and it is within Discord's
    /// [limits](crate::types::limits)
    pub fn check(&self) -> Result<(), ViewError> {
        if let Some(error) = self.extras.custom_id_error() {
            return Err(ViewError::CustomId(error.clone()));
        }
        limits::check_message(&self.components)
    }

    /// The components, or the error building one of them. Drops the files and flags; send a
    /// View through REST with the bodies' `TryFrom<&View>` (`CreateMessage::try_from(&view)`)
    /// instead.
    pub fn into_components(self) -> Result<Vec<Component>, CustomIdError> {
        match self.extras.into_inner().error {
            Some(error) => Err(error),
            None => Ok(self.components),
        }
    }

    /// The whole message, checked: components, `IS_COMPONENTS_V2` and the other flags, allowed
    /// mentions, files
    pub fn into_message_data(self) -> Result<MessageCallbackData, ViewError> {
        self.check()?;
        let mut data = MessageCallbackData::components_v2(self.components);
        let extras = self.extras.into_inner();
        data.flags = Some(MessageFlags::IsComponentsV2 | extras.flags);
        data.allowed_mentions = extras.allowed_mentions;
        data.files = extras.files;
        Ok(data)
    }
}

impl From<Vec<Component>> for View {
    fn from(components: Vec<Component>) -> Self {
        Self::new(components)
    }
}

/// A Components V2 message, with the View's flags, mentions and files
impl IntoResponse for View {
    fn into_response(self) -> InteractionResponse {
        self.try_into_response()
            .unwrap_or_else(|error| error.response())
    }

    fn try_into_response(self) -> Result<InteractionResponse, HandlerError> {
        self.into_message_data()
            .map(InteractionResponse::ChannelMessageWithSource)
            .map_err(HandlerError::new)
    }
}

/// Why a [`View`] or [`ModalView`] cannot be sent
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ViewError {
    /// A `custom_id` could not be built
    CustomId(CustomIdError),
    /// Past one of Discord's [limits](crate::types::limits)
    Limit {
        /// What was counted, e.g. `"components in a message"`
        what: &'static str,
        /// The fewest allowed
        min: usize,
        /// The most allowed
        max: usize,
        /// How many there were
        found: usize,
    },
    /// Two of something Discord requires to be unique share a value
    Duplicate {
        /// What repeats, e.g. `"option value in a select"`
        what: &'static str,
        /// The repeated value
        value: String,
    },
}

impl Display for ViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewError::CustomId(error) => error.fmt(f),
            ViewError::Limit {
                what,
                min: 0,
                max,
                found,
            } => write!(f, "{found} {what}, Discord allows at most {max}"),
            ViewError::Limit {
                what,
                min,
                max,
                found,
            } => write!(f, "{found} {what}, Discord allows {min} to {max}"),
            ViewError::Duplicate { what, value } => {
                write!(
                    f,
                    "{what} {value:?} repeats, Discord requires each to be unique"
                )
            }
        }
    }
}

impl std::error::Error for ViewError {}

impl From<CustomIdError> for ViewError {
    fn from(error: CustomIdError) -> Self {
        ViewError::CustomId(error)
    }
}

/// A rendered modal, built by `rsx!` from a `<Modal>` root
#[derive(Debug, Clone)]
#[must_use]
pub struct ModalView {
    pub(crate) data: ModalCallbackData,
    pub(crate) error: Option<CustomIdError>,
}

impl ModalView {
    /// The modal: title, `custom_id`, components
    pub fn data(&self) -> &ModalCallbackData {
        &self.data
    }

    /// Whether the modal can be opened: its `custom_id`s were built and it is within Discord's
    /// [limits](crate::types::limits)
    pub fn check(&self) -> Result<(), ViewError> {
        if let Some(error) = &self.error {
            return Err(ViewError::CustomId(error.clone()));
        }
        limits::check_modal(&self.data)
    }

    /// The modal, or the error building its `custom_id`
    pub fn into_data(self) -> Result<ModalCallbackData, CustomIdError> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(self.data),
        }
    }
}

/// The modal response that opens it
impl IntoResponse for ModalView {
    fn into_response(self) -> InteractionResponse {
        self.try_into_response()
            .unwrap_or_else(|error| error.response())
    }

    fn try_into_response(self) -> Result<InteractionResponse, HandlerError> {
        self.check().map_err(HandlerError::new)?;
        Ok(InteractionResponse::Modal(self.data))
    }
}

/// What renders to a [`View`]: a View, or a `#[view]`'s Props. Placed in markup, its
/// components go in place and its files, flags and mentions join the enclosing View.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a View",
    label = "not a View",
    note = "a View is `rsx!`'s output or a #[view]'s Props"
)]
pub trait IntoView {
    /// The View
    fn into_view(self) -> View;
}

impl IntoView for View {
    fn into_view(self) -> View {
        self
    }
}

/// What `<File>`, `<Thumbnail>` and `<Item>` show: a URL (or `attachment://<filename>`), a
/// [`File`] to upload with the message (its `attachment://` URL is set for you), or a received
/// [`Attachment`]
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not media",
    note = "use a URL (&str or String), a File to upload with the message, or an &Attachment"
)]
pub trait IntoMedia {
    /// The media's URL; a file is registered on the View being built
    fn into_media(self, extras: &mut Extras) -> String;
}

impl IntoMedia for &str {
    fn into_media(self, _extras: &mut Extras) -> String {
        self.to_string()
    }
}

impl IntoMedia for String {
    fn into_media(self, _extras: &mut Extras) -> String {
        self
    }
}

impl IntoMedia for &String {
    fn into_media(self, _extras: &mut Extras) -> String {
        self.clone()
    }
}

/// Uploaded with the message, shown as `attachment://<name>`
impl IntoMedia for File {
    fn into_media(self, extras: &mut Extras) -> String {
        let url = format!("attachment://{}", self.name);
        extras.inner().files.push(self);
        url
    }
}

/// An attachment Discord already has, by its URL
impl IntoMedia for &Attachment {
    fn into_media(self, _extras: &mut Extras) -> String {
        self.url.clone()
    }
}

/// What a `{expr}` in text-only markup (`<Text>`, a button label, an option) can be: strings,
/// numbers, and `Option`s of them; call `.to_string()` on other `Display` types
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be placed in text",
    note = "use &str, String, a number, or an Option of one; call `.to_string()` on other Display types"
)]
pub trait IntoText {
    /// Appends `self` to the text being built
    fn push_text(self, text: &mut String);
}

impl IntoText for &str {
    fn push_text(self, text: &mut String) {
        text.push_str(self);
    }
}

impl IntoText for String {
    fn push_text(self, text: &mut String) {
        text.push_str(&self);
    }
}

impl IntoText for &String {
    fn push_text(self, text: &mut String) {
        text.push_str(self);
    }
}

impl<T: IntoText> IntoText for Option<T> {
    fn push_text(self, text: &mut String) {
        if let Some(value) = self {
            value.push_text(text);
        }
    }
}

macro_rules! text {
    ($($ty:ty),*) => {$(
        impl IntoText for $ty {
            fn push_text(self, text: &mut String) {
                let _ = write!(text, "{self}");
            }
        }
    )*};
}

text!(
    char,
    u8,
    u16,
    u32,
    u64,
    u128,
    usize,
    i8,
    i16,
    i32,
    i64,
    i128,
    isize,
    f32,
    f64,
    Snowflake,
    std::fmt::Arguments<'_>
);

/// An attribute value in `rsx!` markup, converted to the component field's type: `"text"` for
/// a `String` or `Option<String>`, a `T` for an `Option<T>`, an emoji as `"👍"`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid value for this attribute",
    note = "the attribute expects `{T}`"
)]
pub trait IntoAttr<T> {
    /// The field value
    fn into_attr(self) -> T;
}

impl<T> IntoAttr<T> for T {
    fn into_attr(self) -> T {
        self
    }
}

impl<T> IntoAttr<Option<T>> for T {
    fn into_attr(self) -> Option<T> {
        Some(self)
    }
}

impl IntoAttr<String> for &str {
    fn into_attr(self) -> String {
        self.to_string()
    }
}

impl IntoAttr<Option<String>> for &str {
    fn into_attr(self) -> Option<String> {
        Some(self.to_string())
    }
}

impl IntoAttr<String> for &String {
    fn into_attr(self) -> String {
        self.clone()
    }
}

impl IntoAttr<Option<String>> for &String {
    fn into_attr(self) -> Option<String> {
        Some(self.clone())
    }
}

/// A unicode emoji
impl IntoAttr<Option<PartialEmoji>> for &str {
    fn into_attr(self) -> Option<PartialEmoji> {
        Some(PartialEmoji {
            id: None,
            name: Some(self.to_string()),
            animated: None,
        })
    }
}

/// What an inline handler can return to re-render: a View or a message View's Props (answered
/// as an update of the message the component is on), or a `Result` of one
#[diagnostic::on_unimplemented(message = "`{Self}` is not a message View")]
pub trait Rerender {
    /// The response updating the component's message with this View
    fn rerender(self) -> Result<InteractionResponse, HandlerError>;
}

impl Rerender for View {
    fn rerender(self) -> Result<InteractionResponse, HandlerError> {
        Update(self).try_into_response()
    }
}

impl<T: Rerender, E: Display + 'static> Rerender for Result<T, E> {
    fn rerender(self) -> Result<InteractionResponse, HandlerError> {
        self.map_err(super::response::into_handler_error)?
            .rerender()
    }
}

/// An inline handler's output, answered by [`Rerender`] when it is one, else as a
/// [`CommandOutput`]; picked by method resolution in the code `#[view]` generates:
/// `(&&HandlerOutput::new(out)).answer()`.
#[doc(hidden)]
pub struct HandlerOutput<T>(Cell<Option<T>>);

impl<T> HandlerOutput<T> {
    pub fn new(output: T) -> Self {
        Self(Cell::new(Some(output)))
    }

    fn take(&self) -> T {
        self.0.take().expect("handler output answered once")
    }
}

#[doc(hidden)]
pub trait ViaRerender {
    fn answer(&self) -> Result<InteractionResponse, HandlerError>;
}

impl<T: Rerender> ViaRerender for &HandlerOutput<T> {
    fn answer(&self) -> Result<InteractionResponse, HandlerError> {
        self.take().rerender()
    }
}

#[doc(hidden)]
pub trait ViaOutput {
    fn answer(&self) -> Result<InteractionResponse, HandlerError>;
}

impl<T: CommandOutput> ViaOutput for HandlerOutput<T> {
    fn answer(&self) -> Result<InteractionResponse, HandlerError> {
        self.take().into_result()
    }
}

/// A View as the `#[app]` listing it sees it: its `custom_id` prefix, whether it has inline handlers, and
/// the Views its markup places, so a forgotten sub-View fails to compile
#[derive(Debug, Clone, Copy)]
pub struct ViewInfo {
    /// The `custom_id` prefix of its inline handlers
    pub prefix: &'static str,
    /// Whether it has inline handlers, so must be listed in an App
    pub handled: bool,
    /// The Views its markup places
    pub views: &'static [ViewInfo],
}

/// Implemented by `#[view]`
pub trait ViewDefinition {
    /// What the App listing it checks
    const INFO: ViewInfo;
}

/// Runs a View's inline handlers; `None` when the handler named in the `custom_id` is not one of
/// them. Implemented by `#[view]`; Views with handlers are listed in an `#[app]`.
pub trait ViewHandler<S>: ViewDefinition {
    /// Runs the `on:click` or `on:select` handler named in the `custom_id`. `fields` is the
    /// `custom_id` after `prefix:`: the handler name, then its captured Props
    fn component(
        _ctx: ComponentCtx<'_, S>,
        _fields: Option<&str>,
    ) -> impl Future<Output = Option<Result<InteractionResponse, HandlerError>>> + Send {
        async { None }
    }

    /// Runs the `on:submit` handler named in the `custom_id`; `fields` as for [`component`](Self::component)
    fn modal(
        _ctx: ModalCtx<'_, S>,
        _fields: Option<&str>,
    ) -> impl Future<Output = Option<Result<InteractionResponse, HandlerError>>> + Send {
        async { None }
    }
}

/// A typed select handler parameter (`on:select:name=|ctx, picked: Vec<Color>| ..`): the
/// selected values as `String`, `Snowflake`, resolved `User`, `Role` or `PartialChannel`, or a
/// `#[derive(Choice)]` enum; one (required), `Option` or `Vec`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot hold select values",
    note = "use String, Snowflake, User, Role, PartialChannel, Attachment, or a #[derive(Choice)] enum, optionally in Option or Vec"
)]
pub trait SelectValue: Sized {
    /// Several selections (a `Vec`)
    const MANY: bool = false;

    /// Reads the selected values; an error message when they do not fit the type
    fn extract(data: &MessageComponentData) -> Result<Self, String>;
}

fn selected(data: &MessageComponentData) -> &[String] {
    data.values.as_deref().unwrap_or_default()
}

macro_rules! select_value {
    ($ty:ty, |$value:ident, $data:ident| $parse:expr) => {
        impl SelectValue for $ty {
            fn extract(data: &MessageComponentData) -> Result<Self, String> {
                let parse =
                    |$value: &str, $data: &MessageComponentData| -> Result<$ty, String> { $parse };
                match selected(data).first() {
                    Some(value) => parse(value, data),
                    None => Err("nothing selected".to_string()),
                }
            }
        }

        impl SelectValue for Option<$ty> {
            fn extract(data: &MessageComponentData) -> Result<Self, String> {
                match selected(data).first() {
                    Some(_) => <$ty as SelectValue>::extract(data).map(Some),
                    None => Ok(None),
                }
            }
        }

        impl SelectValue for Vec<$ty> {
            const MANY: bool = true;

            fn extract(data: &MessageComponentData) -> Result<Self, String> {
                let parse =
                    |$value: &str, $data: &MessageComponentData| -> Result<$ty, String> { $parse };
                selected(data)
                    .iter()
                    .map(|value| parse(value, data))
                    .collect()
            }
        }
    };
}

select_value!(String, |value, _data| Ok(value.to_string()));

select_value!(Snowflake, |value, _data| value
    .parse()
    .map_err(|_| format!("`{value}` is not an id")));

select_value!(User, |value, data| resolve(value, data, |r| r
    .users
    .as_ref()));

select_value!(Role, |value, data| resolve(value, data, |r| r
    .roles
    .as_ref()));

select_value!(PartialChannel, |value, data| resolve(value, data, |r| r
    .channels
    .as_ref()));

select_value!(Attachment, |value, data| resolve(value, data, |r| r
    .attachments
    .as_ref()));

fn resolve<T: Clone>(
    value: &str,
    data: &MessageComponentData,
    get: impl Fn(&crate::types::ResolvedData) -> Option<&crate::types::SnowflakeMap<T>>,
) -> Result<T, String> {
    let id: Snowflake = value
        .parse()
        .map_err(|_| format!("`{value}` is not an id"))?;
    data.resolved
        .as_ref()
        .and_then(get)
        .and_then(|map| map.get(&id))
        .cloned()
        .ok_or_else(|| format!("{id} is not in the resolved data"))
}

fn choice<C: ModalChoice>(value: &str) -> Result<C, String> {
    C::from_modal_value(value).ok_or_else(|| format!("`{value}` is not a choice"))
}

impl<C: ModalChoice> SelectValue for C {
    fn extract(data: &MessageComponentData) -> Result<Self, String> {
        match selected(data).first() {
            Some(value) => choice(value),
            None => Err("nothing selected".to_string()),
        }
    }
}

impl<C: ModalChoice> SelectValue for Option<C> {
    fn extract(data: &MessageComponentData) -> Result<Self, String> {
        selected(data)
            .first()
            .map(|value| choice(value))
            .transpose()
    }
}

impl<C: ModalChoice> SelectValue for Vec<C> {
    const MANY: bool = true;

    fn extract(data: &MessageComponentData) -> Result<Self, String> {
        selected(data).iter().map(|value| choice(value)).collect()
    }
}

/// Reads a typed select handler parameter; used by `#[view]`
#[doc(hidden)]
pub fn select_value<T: SelectValue>(data: &MessageComponentData) -> Result<T, HandlerError> {
    T::extract(data).map_err(|e| HandlerError::invalid_option(&data.custom_id, e))
}

/// Reads a typed modal submit parameter; used by `#[view]`
#[doc(hidden)]
pub fn modal_value<T: super::ModalValue>(
    data: &crate::types::ModalSubmitData,
    name: &str,
) -> Result<T, HandlerError> {
    T::extract(data.find(name), data).map_err(|e| HandlerError::invalid_option(name, e))
}

/// The options of a select or group from a `#[derive(Choice)]` enum, as `(label, value)`
#[doc(hidden)]
pub fn choice_options<C: ModalChoice>() -> &'static [(&'static str, &'static str)] {
    C::OPTIONS
}

/// `required: false` for an input whose submit parameter is optional
#[doc(hidden)]
pub const fn required<T: super::ModalValue>() -> Option<bool> {
    if T::REQUIRED { None } else { Some(false) }
}
