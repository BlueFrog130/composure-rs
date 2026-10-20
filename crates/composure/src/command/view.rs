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
    fmt::{Display, Write},
};

use crate::types::{
    Attachment, Component, InteractionResponse, MessageComponentData, ModalCallbackData,
    PartialChannel, PartialEmoji, Role, Snowflake, User,
};

use super::{
    CommandOutput, ComponentCtx, HandlerError, IntoResponse, ModalChoice, ModalCtx, Update,
    custom_id::CustomIdError,
};

/// A rendered message layout: Components V2 components, answered as a new message (or, from an
/// inline handler, as an update of the message the component is on). Built by
/// [`rsx!`](crate::rsx); see [Views](crate::guide::views).
///
/// A `custom_id` that could not be built (state past 100 characters) is kept inside and surfaces
/// as a [`HandlerError`] when the View is answered with.
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct View {
    components: Vec<Component>,
    error: Option<CustomIdError>,
}

impl View {
    /// A View of `components`, as built by hand
    pub fn new(components: Vec<Component>) -> Self {
        Self {
            components,
            error: None,
        }
    }

    /// The top-level components
    pub fn components(&self) -> &[Component] {
        &self.components
    }

    /// The components, or the error building one of them; for messages sent through REST
    pub fn into_components(self) -> Result<Vec<Component>, CustomIdError> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(self.components),
        }
    }
}

impl From<Vec<Component>> for View {
    fn from(components: Vec<Component>) -> Self {
        Self::new(components)
    }
}

/// A Components V2 message
impl IntoResponse for View {
    fn into_response(self) -> InteractionResponse {
        self.try_into_response()
            .unwrap_or_else(|error| error.response())
    }

    fn try_into_response(self) -> Result<InteractionResponse, HandlerError> {
        self.into_components()
            .map(InteractionResponse::respond_with_components)
            .map_err(HandlerError::new)
    }
}

/// A rendered modal, built by `rsx!` from a `<Modal>` root
#[derive(Debug, Clone)]
#[must_use]
pub struct ModalView {
    data: ModalCallbackData,
    error: Option<CustomIdError>,
}

impl ModalView {
    /// The modal: title, `custom_id`, components
    pub fn data(&self) -> &ModalCallbackData {
        &self.data
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
        self.into_data()
            .map(InteractionResponse::Modal)
            .map_err(HandlerError::new)
    }
}

/// Components being collected by `rsx!` for one level of markup: adjacent text joins into one
/// [`TextDisplay`](crate::types::TextDisplay), and the first `custom_id` error is kept.
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct Children {
    components: Vec<Component>,
    text: Option<String>,
    error: Option<CustomIdError>,
}

impl Children {
    pub fn text(&mut self, text: &str) {
        self.text.get_or_insert_with(String::new).push_str(text);
    }

    pub fn component(&mut self, component: Component) {
        self.flush();
        self.components.push(component);
    }

    pub fn error(&mut self, error: Option<CustomIdError>) {
        if self.error.is_none() {
            self.error = error;
        }
    }

    /// A `custom_id`, or an empty one with the error kept
    pub fn id(&mut self, custom_id: Result<String, CustomIdError>) -> String {
        custom_id.unwrap_or_else(|error| {
            self.error(Some(error));
            String::new()
        })
    }

    /// The components of a nested level, taking over its error
    pub fn nested(&mut self, mut children: Children) -> Vec<Component> {
        children.flush();
        self.error(children.error);
        children.components
    }

    /// The text collected so far (text-only markup: button labels, options)
    pub fn take_text(&mut self) -> String {
        self.text.take().unwrap_or_default()
    }

    fn flush(&mut self) {
        if let Some(text) = self.text.take() {
            self.components.push(Component::text(text));
        }
    }

    pub fn view(mut self) -> View {
        self.flush();
        View {
            components: self.components,
            error: self.error,
        }
    }

    pub fn modal(mut self, mut data: ModalCallbackData) -> ModalView {
        self.flush();
        data.components = Some(self.components);
        ModalView {
            data,
            error: self.error,
        }
    }
}

/// What a `{expr}` in `rsx!` markup can be: text (strings, numbers), components, Views and
/// Props, and `Option`s, `Vec`s, arrays and iterator adapters (`map`, `filter`, ...) of those.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be placed in `rsx!` markup",
    note = "use text (&str, String, numbers), a Component, a View, a #[view]'s Props, or an Option, Vec, or iterator of them; collect other iterators into a Vec"
)]
pub trait IntoChildren {
    /// Adds `self` to the markup level being built
    fn push_into(self, children: &mut Children);
}

impl IntoChildren for View {
    fn push_into(self, children: &mut Children) {
        children.error(self.error);
        for component in self.components {
            children.component(component);
        }
    }
}

impl IntoChildren for Component {
    fn push_into(self, children: &mut Children) {
        children.component(self);
    }
}

macro_rules! into_children_component {
    ($($ty:ty),*) => {$(
        impl IntoChildren for $ty {
            fn push_into(self, children: &mut Children) {
                children.component(self.into());
            }
        }
    )*};
}

into_children_component!(
    crate::types::ActionRow,
    crate::types::ButtonComponent,
    crate::types::Container,
    crate::types::Section,
    crate::types::TextDisplay,
    crate::types::Thumbnail,
    crate::types::MediaGallery,
    crate::types::FileComponent,
    crate::types::Separator,
    crate::types::Label
);

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

macro_rules! into_children_text {
    ($($ty:ty),*) => {$(
        impl IntoChildren for $ty {
            fn push_into(self, children: &mut Children) {
                let text = children.text.get_or_insert_with(String::new);
                IntoText::push_text(self, text);
            }
        }
    )*};
}

into_children_text!(
    &str,
    String,
    &String,
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

impl<T: IntoChildren> IntoChildren for Option<T> {
    fn push_into(self, children: &mut Children) {
        if let Some(child) = self {
            child.push_into(children);
        }
    }
}

impl<T: IntoChildren> IntoChildren for Vec<T> {
    fn push_into(self, children: &mut Children) {
        for child in self {
            child.push_into(children);
        }
    }
}

impl<T: IntoChildren, const N: usize> IntoChildren for [T; N] {
    fn push_into(self, children: &mut Children) {
        for child in self {
            child.push_into(children);
        }
    }
}

macro_rules! into_children_iter {
    ($($ty:ident<$($param:ident),*>),*) => {$(
        impl<$($param),*> IntoChildren for ::core::iter::$ty<$($param),*>
        where
            ::core::iter::$ty<$($param),*>: Iterator<Item: IntoChildren>,
        {
            fn push_into(self, children: &mut Children) {
                for child in self {
                    child.push_into(children);
                }
            }
        }
    )*};
}

into_children_iter!(
    Map<I, F>,
    Filter<I, P>,
    FilterMap<I, F>,
    Chain<A, B>,
    Take<I>,
    Skip<I>,
    Rev<I>,
    Once<T>
);

impl<I: Iterator, U: IntoIterator<Item: IntoChildren>, F: FnMut(I::Item) -> U> IntoChildren
    for ::core::iter::FlatMap<I, U, F>
{
    fn push_into(self, children: &mut Children) {
        for child in self {
            child.push_into(children);
        }
    }
}

impl<I: Iterator<Item: IntoIterator<Item: IntoChildren>>> IntoChildren
    for ::core::iter::Flatten<I>
{
    fn push_into(self, children: &mut Children) {
        for child in self {
            child.push_into(children);
        }
    }
}

impl<T: IntoChildren> IntoChildren for std::vec::IntoIter<T> {
    fn push_into(self, children: &mut Children) {
        for child in self {
            child.push_into(children);
        }
    }
}

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
