//! What `rsx!` markup expands to: a builder per built-in tag. A tag is its component type's
//! `builder()`, each attribute a documented method, each child a `.child(..)` whose bound says
//! where it may go, and `.build()` the [`Element`] the parent takes:
//!
//! ```ignore
//! <Row><Button primary on:click=like(id)>"Like"</Button></Row>
//! // is
//! ActionRow::builder()
//!     .child(ButtonComponent::builder().primary(true).on_click(like::custom_id(id)).child("Like").build())
//!     .build()
//! ```
//!
//! So the compiler checks attributes (a method per attribute) and placement (a trait per
//! parent: text in a `<Row>` does not implement [`RowChild`]), and the IDE completes and
//! documents both. The builders also work by hand.

use std::fmt;

use crate::types::{
    ActionRow, AllowedMentions, ButtonComponent, ButtonStyle, ChannelType, Checkbox, CheckboxGroup,
    Component, Container, File, FileComponent, FileUpload, GroupOption, Label, MediaGallery,
    MediaGalleryItem, MessageFlags, ModalCallbackData, PartialEmoji, RadioGroup, Section,
    SelectDefaultValue, SelectMenu, SelectOption, Separator, SeparatorSpacing, Snowflake,
    TextDisplay, TextInput, TextInputStyle, Thumbnail, UnfurledMedia,
};

use super::{
    IntoAttr, IntoMedia, IntoText, IntoView, ModalChoice, ModalView, View, custom_id::CustomIdError,
};

/// What a built tag brings to its message besides its component: files to upload, the flags and
/// mentions of placed Views, the first `custom_id` that could not be built. Carried up to the
/// [`View`] (or [`ModalView`]) the markup builds. Boxed, and only when there is something, so
/// the builders and elements that carry it stay small.
#[doc(hidden)]
#[derive(Debug, Clone, Default)]
pub struct Extras(Option<Box<ExtrasInner>>);

#[derive(Debug, Clone)]
pub(crate) struct ExtrasInner {
    pub(crate) files: Vec<File>,
    pub(crate) flags: MessageFlags,
    pub(crate) allowed_mentions: Option<AllowedMentions>,
    pub(crate) error: Option<CustomIdError>,
}

impl Default for ExtrasInner {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            flags: MessageFlags::empty(),
            allowed_mentions: None,
            error: None,
        }
    }
}

impl Extras {
    pub(crate) fn inner(&mut self) -> &mut ExtrasInner {
        self.0.get_or_insert_with(Default::default)
    }

    pub(crate) fn into_inner(self) -> ExtrasInner {
        self.0.map(|inner| *inner).unwrap_or_default()
    }

    pub(crate) fn files(&self) -> &[File] {
        self.0.as_ref().map_or(&[], |inner| &inner.files)
    }

    pub(crate) fn flags(&self) -> MessageFlags {
        self.0
            .as_ref()
            .map_or(MessageFlags::empty(), |inner| inner.flags)
    }

    pub(crate) fn mentions(&self) -> Option<&AllowedMentions> {
        self.0
            .as_ref()
            .and_then(|inner| inner.allowed_mentions.as_ref())
    }

    pub(crate) fn custom_id_error(&self) -> Option<&CustomIdError> {
        self.0.as_ref().and_then(|inner| inner.error.as_ref())
    }

    /// Takes over a child's: files appended, flags joined, the first mentions and error kept
    pub(crate) fn merge(&mut self, other: Extras) {
        let Some(other) = other.0 else {
            return;
        };
        let Some(this) = &mut self.0 else {
            self.0 = Some(other);
            return;
        };
        let other = *other;
        if this.error.is_none() {
            this.error = other.error;
        }
        this.files.extend(other.files);
        this.flags |= other.flags;
        if this.allowed_mentions.is_none() {
            this.allowed_mentions = other.allowed_mentions;
        }
    }

    pub(crate) fn error(&mut self, error: CustomIdError) {
        self.inner().error.get_or_insert(error);
    }
}

/// A built tag: its component (or option, or item) and its [`Extras`], for the parent's
/// `.child(..)`
#[doc(hidden)]
#[derive(Debug)]
pub struct Element<T> {
    pub(crate) value: T,
    pub(crate) extras: Extras,
}

impl<T> Element<T> {
    fn new(value: T, extras: Extras) -> Self {
        Self { value, extras }
    }

    /// The built value, dropping its files and error (for hand-built markup)
    pub fn into_inner(self) -> T {
        self.value
    }
}

/// A `custom_id`: a string, or what a handler's `custom_id(..)` returns, whose error goes to the
/// message (and surfaces when it is answered with)
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a custom_id",
    note = "use a String, a &str, or a handler's `custom_id(..)`"
)]
pub trait IntoCustomId {
    /// The `custom_id`, empty with the error kept when it could not be built
    fn into_custom_id(self, extras: &mut Extras) -> String;
}

impl IntoCustomId for String {
    fn into_custom_id(self, _extras: &mut Extras) -> String {
        self
    }
}

impl IntoCustomId for &str {
    fn into_custom_id(self, _extras: &mut Extras) -> String {
        self.to_string()
    }
}

impl IntoCustomId for &String {
    fn into_custom_id(self, _extras: &mut Extras) -> String {
        self.clone()
    }
}

impl IntoCustomId for Result<String, CustomIdError> {
    fn into_custom_id(self, extras: &mut Extras) -> String {
        self.unwrap_or_else(|error| {
            extras.error(error);
            String::new()
        })
    }
}

/// The components of a markup level with text: adjacent text joins into one text display
#[doc(hidden)]
#[derive(Debug, Default)]
pub struct Kids {
    components: Vec<Component>,
    text: Option<String>,
    extras: Extras,
}

impl Kids {
    fn reserve(&mut self, additional: usize) {
        self.components.reserve(additional);
    }

    fn text(&mut self, text: impl IntoText) {
        text.push_text(self.text.get_or_insert_with(String::new));
    }

    fn component(&mut self, component: Component) {
        self.flush();
        self.components.push(component);
    }

    fn element<T: Into<Component>>(&mut self, element: Element<T>) {
        self.extras.merge(element.extras);
        self.component(element.value.into());
    }

    fn view(&mut self, view: View) {
        self.extras.merge(view.extras);
        for component in view.components {
            self.component(component);
        }
    }

    fn flush(&mut self) {
        if let Some(text) = self.text.take() {
            self.components.push(Component::text(text));
        }
    }

    fn finish(mut self) -> (Vec<Component>, Extras) {
        self.flush();
        (self.components, self.extras)
    }
}

/// Items of a gallery or options of a select or group
#[doc(hidden)]
#[derive(Debug)]
pub struct List<T> {
    items: Vec<T>,
    extras: Extras,
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            extras: Extras::default(),
        }
    }
}

/// `Option`, `Vec`, arrays and iterator adapters of children are children of the same parent
macro_rules! collections {
    ($trait:ident, $target:ty) => {
        impl<T: $trait> $trait for Option<T> {
            fn add_to(self, parent: &mut $target) {
                if let Some(child) = self {
                    child.add_to(parent);
                }
            }
        }

        impl<T: $trait> $trait for Vec<T> {
            fn add_to(self, parent: &mut $target) {
                for child in self {
                    child.add_to(parent);
                }
            }
        }

        impl<T: $trait, const N: usize> $trait for [T; N] {
            fn add_to(self, parent: &mut $target) {
                for child in self {
                    child.add_to(parent);
                }
            }
        }

        impl<T: $trait> $trait for ::std::vec::IntoIter<T> {
            fn add_to(self, parent: &mut $target) {
                for child in self {
                    child.add_to(parent);
                }
            }
        }

        collections!(@iter $trait, $target, Map<I, F>, Filter<I, P>, FilterMap<I, F>, Chain<A, B>,
            Take<I>, Skip<I>, Rev<I>, Once<T>);

        impl<I: Iterator, U: IntoIterator<Item: $trait>, F: FnMut(I::Item) -> U> $trait
            for ::core::iter::FlatMap<I, U, F>
        {
            fn add_to(self, parent: &mut $target) {
                for child in self {
                    child.add_to(parent);
                }
            }
        }

        impl<I: Iterator<Item: IntoIterator<Item: $trait>>> $trait for ::core::iter::Flatten<I> {
            fn add_to(self, parent: &mut $target) {
                for child in self {
                    child.add_to(parent);
                }
            }
        }
    };
    (@iter $trait:ident, $target:ty, $($ty:ident<$($param:ident),*>),*) => {$(
        impl<$($param),*> $trait for ::core::iter::$ty<$($param),*>
        where
            ::core::iter::$ty<$($param),*>: Iterator<Item: $trait>,
        {
            fn add_to(self, parent: &mut $target) {
                for child in self {
                    child.add_to(parent);
                }
            }
        }
    )*};
}

macro_rules! child_trait {
    ($(#[$doc:meta])* $trait:ident, $target:ty, $message:literal, $note:literal) => {
        $(#[$doc])*
        #[diagnostic::on_unimplemented(message = $message, note = $note)]
        pub trait $trait {
            /// Adds `self` to the parent being built
            #[doc(hidden)]
            fn add_to(self, parent: &mut $target);
        }

        collections!($trait, $target);
    };
}

child_trait!(
    /// What can go at the top of a message, or in a sub-View's children: text, `<Text>`,
    /// `<Container>`, `<Section>`, `<Gallery>`, `<File>`, `<Separator>`, `<Row>`, a single
    /// `<Button>` or select (a sub-View meant for a row), components, Views and Props
    MessageChild,
    Kids,
    "`{Self}` cannot go at the top of a message",
    "use text, a <Text>, <Container>, <Section>, <Gallery>, <File>, <Separator> or <Row>, a Component, a View or a #[view]'s Props, or an Option, Vec or iterator of them"
);

child_trait!(
    /// What can go in a `<Container>`: as [`MessageChild`] but another `<Container>`
    ContainerChild,
    Kids,
    "`{Self}` cannot go in a `<Container>`",
    "a <Container> takes text, <Text>, <Section>, <Gallery>, <File>, <Separator> and <Row>, components, Views and Props; buttons and selects go in a <Row>"
);

child_trait!(
    /// What can go in a `<Section>`: 1-3 texts, and one accessory, a `<Button>` or `<Thumbnail>`
    SectionChild,
    SectionBuilder,
    "`{Self}` cannot go in a `<Section>`",
    "a <Section> takes 1-3 texts or <Text>s, and one <Button> or <Thumbnail> as its accessory"
);

child_trait!(
    /// What can go in a `<Row>`: up to 5 `<Button>`s, or one select
    RowChild,
    Kids,
    "`{Self}` cannot go in a `<Row>`",
    "a <Row> takes up to 5 <Button>s, or one select"
);

child_trait!(
    /// What can go in a `<Modal>`: text, `<Text>` and `<Label>`s, at most 5
    ModalChild,
    Kids,
    "`{Self}` cannot go in a `<Modal>`",
    "a <Modal> takes text, <Text> and <Label>s (each around one input)"
);

child_trait!(
    /// What can go in a `<Label>`: one input
    LabelChild,
    LabelBuilder,
    "`{Self}` cannot go in a `<Label>`",
    "a <Label> holds one input: <TextInput>, a select, <RadioGroup>, <CheckboxGroup>, <Checkbox> or <FileUpload>"
);

child_trait!(
    /// What can go in a `<Gallery>`: `<Item>`s
    GalleryChild,
    List<MediaGalleryItem>,
    "`{Self}` cannot go in a `<Gallery>`",
    "a <Gallery> takes <Item>s and MediaGalleryItems"
);

child_trait!(
    /// What can go in a `<StringSelect>`: `<Option>`s
    SelectOptionChild,
    List<SelectOption>,
    "`{Self}` cannot go in a select",
    "a select takes <Option>s and SelectOptions"
);

child_trait!(
    /// What can go in a `<RadioGroup>` or `<CheckboxGroup>`: `<Option>`s
    GroupOptionChild,
    List<GroupOption>,
    "`{Self}` cannot go in a group",
    "a <RadioGroup> or <CheckboxGroup> takes <Option>s and GroupOptions"
);

// Text: joined with adjacent text into one text display
macro_rules! text_child {
    ($($ty:ty),*) => {$(
        impl MessageChild for $ty {
            fn add_to(self, parent: &mut Kids) {
                parent.text(self);
            }
        }

        impl ContainerChild for $ty {
            fn add_to(self, parent: &mut Kids) {
                parent.text(self);
            }
        }

        impl SectionChild for $ty {
            fn add_to(self, parent: &mut SectionBuilder) {
                parent.kids.text(self);
            }
        }

        impl ModalChild for $ty {
            fn add_to(self, parent: &mut Kids) {
                parent.text(self);
            }
        }
    )*};
}

text_child!(
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
    fmt::Arguments<'_>
);

// Views and Props: their components in place, their extras joined
macro_rules! view_child {
    ($($trait:ident),*) => {$(
        #[diagnostic::do_not_recommend]
        impl<V: IntoView> $trait for V {
            fn add_to(self, parent: &mut Kids) {
                parent.view(self.into_view());
            }
        }
    )*};
}

view_child!(MessageChild, ContainerChild, RowChild, ModalChild);

#[diagnostic::do_not_recommend]
impl<V: IntoView> SectionChild for V {
    fn add_to(self, parent: &mut SectionBuilder) {
        parent.kids.view(self.into_view());
    }
}

// Components as values: any component (checked by Discord's limits when answered with)
macro_rules! component_child {
    ($($trait:ident),*) => {$(
        impl $trait for Component {
            fn add_to(self, parent: &mut Kids) {
                parent.component(self);
            }
        }
    )*};
}

component_child!(MessageChild, ContainerChild, RowChild, ModalChild);

impl SectionChild for Component {
    fn add_to(self, parent: &mut SectionBuilder) {
        parent.kids.component(self);
    }
}

impl LabelChild for Component {
    fn add_to(self, parent: &mut LabelBuilder) {
        parent.input = Some(self);
    }
}

/// Built tags and their component types, where Discord allows them
macro_rules! placed {
    ($ty:ty => $($trait:ident),*) => {$(
        impl $trait for Element<$ty> {
            fn add_to(self, parent: &mut Kids) {
                parent.element(self);
            }
        }

        impl $trait for $ty {
            fn add_to(self, parent: &mut Kids) {
                parent.component(self.into());
            }
        }
    )*};
}

placed!(TextDisplay => MessageChild, ContainerChild, ModalChild);
placed!(Container => MessageChild);
placed!(Section => MessageChild, ContainerChild);
placed!(MediaGallery => MessageChild, ContainerChild);
placed!(FileComponent => MessageChild, ContainerChild);
placed!(Separator => MessageChild, ContainerChild);
placed!(ActionRow => MessageChild, ContainerChild);
placed!(ButtonComponent => MessageChild, RowChild);
placed!(Label => ModalChild);

macro_rules! select_placed {
    ($($n:literal),*) => {$(
        placed!(SelectMenu<$n> => MessageChild, RowChild);

        impl LabelChild for Element<SelectMenu<$n>> {
            fn add_to(self, parent: &mut LabelBuilder) {
                parent.extras.merge(self.extras);
                parent.input = Some(self.value.into());
            }
        }

        impl LabelChild for SelectMenu<$n> {
            fn add_to(self, parent: &mut LabelBuilder) {
                parent.input = Some(self.into());
            }
        }
    )*};
}

select_placed!(3, 5, 6, 7, 8);

impl SectionChild for Element<TextDisplay> {
    fn add_to(self, parent: &mut SectionBuilder) {
        parent.kids.element(self);
    }
}

impl SectionChild for TextDisplay {
    fn add_to(self, parent: &mut SectionBuilder) {
        parent.kids.component(self.into());
    }
}

macro_rules! accessory {
    ($($ty:ty),*) => {$(
        impl SectionChild for Element<$ty> {
            fn add_to(self, parent: &mut SectionBuilder) {
                parent.kids.extras.merge(self.extras);
                parent.accessory = Some(self.value.into());
            }
        }

        impl SectionChild for $ty {
            fn add_to(self, parent: &mut SectionBuilder) {
                parent.accessory = Some(self.into());
            }
        }
    )*};
}

accessory!(ButtonComponent, Thumbnail);

macro_rules! input {
    ($($ty:ty),*) => {$(
        impl LabelChild for Element<$ty> {
            fn add_to(self, parent: &mut LabelBuilder) {
                parent.extras.merge(self.extras);
                parent.input = Some(self.value.into());
            }
        }

        impl LabelChild for $ty {
            fn add_to(self, parent: &mut LabelBuilder) {
                parent.input = Some(self.into());
            }
        }
    )*};
}

input!(TextInput, RadioGroup, CheckboxGroup, Checkbox, FileUpload);

impl GalleryChild for Element<MediaGalleryItem> {
    fn add_to(self, parent: &mut List<MediaGalleryItem>) {
        parent.extras.merge(self.extras);
        parent.items.push(self.value);
    }
}

impl GalleryChild for MediaGalleryItem {
    fn add_to(self, parent: &mut List<MediaGalleryItem>) {
        parent.items.push(self);
    }
}

/// An `<Option>`, before it knows whether a select or a group takes it
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ChoiceOption {
    label: String,
    value: String,
    description: Option<String>,
    emoji: Option<PartialEmoji>,
    default: Option<bool>,
}

impl SelectOptionChild for Element<ChoiceOption> {
    fn add_to(self, parent: &mut List<SelectOption>) {
        let o = self.value;
        parent.items.push(SelectOption::new(
            o.label,
            o.value,
            o.description,
            o.emoji,
            o.default,
        ));
    }
}

impl SelectOptionChild for SelectOption {
    fn add_to(self, parent: &mut List<SelectOption>) {
        parent.items.push(self);
    }
}

impl GroupOptionChild for Element<ChoiceOption> {
    fn add_to(self, parent: &mut List<GroupOption>) {
        let o = self.value;
        let mut option = GroupOption::new(o.label, o.value);
        option.description = o.description;
        option.default = o.default;
        parent.items.push(option);
    }
}

impl GroupOptionChild for GroupOption {
    fn add_to(self, parent: &mut List<GroupOption>) {
        parent.items.push(self);
    }
}

/// `pub fn name(self, value: impl IntoAttr<T>) -> Self` setting a field of the component
macro_rules! setters {
    ($($(#[$doc:meta])* $name:ident: $ty:ty => $($field:ident).+;)*) => {$(
        $(#[$doc])*
        pub fn $name(mut self, value: impl IntoAttr<$ty>) -> Self {
            self.$($field).+ = value.into_attr();
            self
        }
    )*};
}

/// `.reserve(n)`, emitted by `rsx!` with the number of children it sees, so a level allocates once
macro_rules! reserve {
    ($($field:ident).+) => {
        /// Room for `additional` children
        #[doc(hidden)]
        pub fn reserve(mut self, additional: usize) -> Self {
            self.$($field).+.reserve(additional);
            self
        }
    };
}

/// A View from markup: `rsx!`'s top level, and a sub-View's children
#[derive(Debug, Default)]
#[must_use]
pub struct ViewBuilder {
    kids: Kids,
}

impl View {
    /// A View built child by child, as `rsx!` does
    pub fn builder() -> ViewBuilder {
        ViewBuilder::default()
    }
}

impl ViewBuilder {
    reserve!(kids);

    /// Adds a child: text, a component, a View or Props, or an `Option`, `Vec` or iterator of them
    pub fn child(mut self, child: impl MessageChild) -> Self {
        child.add_to(&mut self.kids);
        self
    }

    /// The View
    pub fn build(self) -> View {
        let (components, extras) = self.kids.finish();
        View { components, extras }
    }
}

/// `<Container>`: a frame around other components, like an embed, with an optional accent bar
#[derive(Debug)]
#[must_use]
pub struct ContainerBuilder {
    el: Container,
    kids: Kids,
}

impl Container {
    /// A container built child by child, as `<Container>` does
    pub fn builder() -> ContainerBuilder {
        ContainerBuilder {
            el: Container::new(Vec::new()),
            kids: Kids::default(),
        }
    }
}

impl ContainerBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `u32` (`0xRRGGBB`): color of the bar on the left; none when unset
        accent_color: Option<u32> => el.accent_color;
        /// `bool`: blurs the content until clicked
        spoiler: Option<bool> => el.spoiler;
    }

    reserve!(kids);

    /// Adds a child: text, `<Text>`, `<Section>`, `<Gallery>`, `<File>`, `<Separator>`, `<Row>`,
    /// a component, a View or Props
    pub fn child(mut self, child: impl ContainerChild) -> Self {
        child.add_to(&mut self.kids);
        self
    }

    /// The container
    pub fn build(mut self) -> Element<Container> {
        let (components, extras) = self.kids.finish();
        self.el.components = components;
        Element::new(self.el, extras)
    }
}

/// `<Section>`: 1-3 text displays beside an accessory
#[derive(Debug)]
#[must_use]
pub struct SectionBuilder {
    id: Option<i32>,
    kids: Kids,
    accessory: Option<Component>,
}

impl Section {
    /// A section built child by child, as `<Section>` does
    pub fn builder() -> SectionBuilder {
        SectionBuilder {
            id: None,
            kids: Kids::default(),
            accessory: None,
        }
    }
}

impl SectionBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => id;
    }

    reserve!(kids);

    /// `impl Into<Component>`: the accessory as a value, instead of a `<Button>` or `<Thumbnail>`
    /// child
    pub fn accessory(mut self, accessory: impl Into<Component>) -> Self {
        self.accessory = Some(accessory.into());
        self
    }

    /// Adds a child: text or `<Text>`, or the accessory, a `<Button>` or `<Thumbnail>`
    pub fn child(mut self, child: impl SectionChild) -> Self {
        child.add_to(&mut self);
        self
    }

    /// The section; without an accessory, a separator stands in (Discord rejects it, as `rsx!`
    /// does at compile time)
    pub fn build(self) -> Element<Section> {
        let (components, extras) = self.kids.finish();
        let accessory = self
            .accessory
            .unwrap_or_else(|| Component::Separator(Separator::new()));
        let mut section = Section::new(components, accessory);
        section.id = self.id;
        Element::new(section, extras)
    }
}

/// `<Text>`: markdown
#[derive(Debug)]
#[must_use]
pub struct TextBuilder {
    el: TextDisplay,
    text: Option<String>,
}

impl TextDisplay {
    /// A text display built from text children, as `<Text>` does
    pub fn builder() -> TextBuilder {
        TextBuilder {
            el: TextDisplay::new(String::new()),
            text: None,
        }
    }
}

impl TextBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `String`: the text, instead of children
        content: String => el.content;
    }

    /// Room for `additional` bytes of text
    #[doc(hidden)]
    pub fn reserve(mut self, additional: usize) -> Self {
        self.text
            .get_or_insert_with(String::new)
            .reserve(additional);
        self
    }

    /// Appends text: a string, a number, or an `Option` of one
    pub fn child(mut self, text: impl IntoText) -> Self {
        text.push_text(self.text.get_or_insert_with(String::new));
        self
    }

    /// The text display
    pub fn build(mut self) -> Element<TextDisplay> {
        if let Some(text) = self.text {
            self.el.content = text;
        }
        Element::new(self.el, Extras::default())
    }
}

/// `src`/`url`, with the media's file (if any) kept for the message
macro_rules! media {
    ($($field:ident).+) => {
        /// The media: a URL (or `attachment://<filename>`), a [`File`] to upload with the
        /// message (its `attachment://` URL is set for you), or an `&Attachment`
        pub fn src(mut self, media: impl IntoMedia) -> Self {
            self.$($field).+ = UnfurledMedia::new(media.into_media(&mut self.extras));
            self
        }

        /// The same as `src`
        pub fn url(self, media: impl IntoMedia) -> Self {
            self.src(media)
        }
    };
}

/// `<Thumbnail>`: a small image, as a `<Section>`'s accessory
#[derive(Debug)]
#[must_use]
pub struct ThumbnailBuilder {
    el: Thumbnail,
    extras: Extras,
}

impl Thumbnail {
    /// A thumbnail built as `<Thumbnail>` does
    pub fn builder() -> ThumbnailBuilder {
        ThumbnailBuilder {
            el: Thumbnail::new(String::new()),
            extras: Extras::default(),
        }
    }
}

impl ThumbnailBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `String`: alt text, max 1024 characters
        description: Option<String> => el.description;
        /// `bool`: blurs the image until clicked
        spoiler: Option<bool> => el.spoiler;
    }

    media!(el.media);

    /// The thumbnail
    pub fn build(self) -> Element<Thumbnail> {
        Element::new(self.el, self.extras)
    }
}

/// `<Gallery>`: a grid of 1-10 images and videos
#[derive(Debug)]
#[must_use]
pub struct GalleryBuilder {
    el: MediaGallery,
    items: List<MediaGalleryItem>,
}

impl MediaGallery {
    /// A gallery built item by item, as `<Gallery>` does
    pub fn builder() -> GalleryBuilder {
        GalleryBuilder {
            el: MediaGallery::new(Vec::new()),
            items: List::default(),
        }
    }
}

impl GalleryBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
    }

    reserve!(items.items);

    /// Adds an `<Item>` (or `MediaGalleryItem`s)
    pub fn child(mut self, item: impl GalleryChild) -> Self {
        item.add_to(&mut self.items);
        self
    }

    /// The gallery
    pub fn build(mut self) -> Element<MediaGallery> {
        self.el.items = self.items.items;
        Element::new(self.el, self.items.extras)
    }
}

/// `<Item>`: one image or video in a `<Gallery>`
#[derive(Debug)]
#[must_use]
pub struct ItemBuilder {
    el: MediaGalleryItem,
    extras: Extras,
}

impl MediaGalleryItem {
    /// A gallery item built as `<Item>` does
    pub fn builder() -> ItemBuilder {
        ItemBuilder {
            el: MediaGalleryItem::new(String::new()),
            extras: Extras::default(),
        }
    }
}

impl ItemBuilder {
    setters! {
        /// `String`: alt text, max 1024 characters
        description: Option<String> => el.description;
        /// `bool`: blurs the media until clicked
        spoiler: Option<bool> => el.spoiler;
    }

    media!(el.media);

    /// The item
    pub fn build(self) -> Element<MediaGalleryItem> {
        Element::new(self.el, self.extras)
    }
}

/// `<File>`: a file uploaded with the message, shown inline
#[derive(Debug)]
#[must_use]
pub struct FileBuilder {
    el: FileComponent,
    extras: Extras,
}

impl FileComponent {
    /// A file component built as `<File>` does
    pub fn builder() -> FileBuilder {
        FileBuilder {
            el: FileComponent::new(String::new()),
            extras: Extras::default(),
        }
    }
}

impl FileBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `bool`: blurs the file until clicked
        spoiler: Option<bool> => el.spoiler;
    }

    media!(el.file);

    /// The file component
    pub fn build(self) -> Element<FileComponent> {
        Element::new(self.el, self.extras)
    }
}

/// `<Separator>`: vertical space, with a divider line by default
#[derive(Debug)]
#[must_use]
pub struct SeparatorBuilder {
    el: Separator,
}

impl Separator {
    /// A separator built as `<Separator>` does
    pub fn builder() -> SeparatorBuilder {
        SeparatorBuilder {
            el: Separator::new(),
        }
    }
}

impl SeparatorBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `SeparatorSpacing`: the spacing, as a value
        spacing: Option<SeparatorSpacing> => el.spacing;
        /// `bool`: `false` for space only, no line
        divider: Option<bool> => el.divider;
    }

    /// Flag: small spacing (the default)
    pub fn small(mut self, small: bool) -> Self {
        if small {
            self.el.spacing = Some(SeparatorSpacing::Small);
        }
        self
    }

    /// Flag: large spacing
    pub fn large(mut self, large: bool) -> Self {
        if large {
            self.el.spacing = Some(SeparatorSpacing::Large);
        }
        self
    }

    /// The separator
    pub fn build(self) -> Element<Separator> {
        Element::new(self.el, Extras::default())
    }
}

/// `<Row>`: up to 5 `<Button>`s, or one select
#[derive(Debug)]
#[must_use]
pub struct RowBuilder {
    el: ActionRow,
    kids: Kids,
}

impl ActionRow {
    /// A row built child by child, as `<Row>` does
    pub fn builder() -> RowBuilder {
        RowBuilder {
            el: ActionRow::new(Vec::new()),
            kids: Kids::default(),
        }
    }
}

impl RowBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
    }

    reserve!(kids);

    /// Adds a `<Button>` or select (or components, Views, Props)
    pub fn child(mut self, child: impl RowChild) -> Self {
        child.add_to(&mut self.kids);
        self
    }

    /// The row
    pub fn build(mut self) -> Element<ActionRow> {
        let (components, extras) = self.kids.finish();
        self.el.components = components;
        Element::new(self.el, extras)
    }
}

/// `<Button>`: in a `<Row>`, or as a `<Section>`'s accessory. Children: the label.
#[derive(Debug)]
#[must_use]
pub struct ButtonBuilder {
    el: ButtonComponent,
    style: Option<ButtonStyle>,
    label: Option<String>,
    extras: Extras,
}

impl ButtonComponent {
    /// A button built as `<Button>` does
    pub fn builder() -> ButtonBuilder {
        ButtonBuilder {
            el: ButtonComponent::new(ButtonStyle::Secondary, None, None, None, None, None),
            style: None,
            label: None,
            extras: Extras::default(),
        }
    }
}

impl ButtonBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `String`: a link button (style `Link`), opening the URL; no `on:click` or `custom_id`
        url: Option<String> => el.url;
        /// `Snowflake`: a premium button (style `Premium`) offering the SKU
        sku_id: Option<Snowflake> => el.sku_id;
        /// `String`: the label, instead of children; max 80 characters
        label: Option<String> => el.label;
        /// `&str` or `PartialEmoji`: shown before the label, `"👍"` or a custom emoji
        emoji: Option<PartialEmoji> => el.emoji;
        /// `bool`: greyed out, not clickable
        disabled: Option<bool> => el.disabled;
    }

    fn flag(mut self, on: bool, style: ButtonStyle) -> Self {
        if on {
            self.style = Some(style);
        }
        self
    }

    /// Flag: blurple
    pub fn primary(self, on: bool) -> Self {
        self.flag(on, ButtonStyle::Primary)
    }

    /// Flag: grey (the default)
    pub fn secondary(self, on: bool) -> Self {
        self.flag(on, ButtonStyle::Secondary)
    }

    /// Flag: green
    pub fn success(self, on: bool) -> Self {
        self.flag(on, ButtonStyle::Success)
    }

    /// Flag: red
    pub fn danger(self, on: bool) -> Self {
        self.flag(on, ButtonStyle::Danger)
    }

    /// `ButtonStyle`: the style, as a value
    pub fn style(mut self, style: impl IntoAttr<ButtonStyle>) -> Self {
        self.style = Some(style.into_attr());
        self
    }

    /// `on:click=handler(state..)`: answered by a `#[component]` handler. `on:click:name=|ctx| ..`
    /// is an inline handler (`#[view]` only).
    pub fn on_click(self, custom_id: impl IntoCustomId) -> Self {
        self.custom_id(custom_id)
    }

    /// `String`: answered by whatever handles this `custom_id`; max 100 characters
    pub fn custom_id(mut self, custom_id: impl IntoCustomId) -> Self {
        self.el.custom_id = Some(custom_id.into_custom_id(&mut self.extras));
        self
    }

    /// Appends to the label: a string, a number, or an `Option` of one
    pub fn child(mut self, text: impl IntoText) -> Self {
        text.push_text(self.label.get_or_insert_with(String::new));
        self
    }

    /// The button: style `Link` with a `url`, `Premium` with a `sku_id`, else its flag or `style`
    /// (grey by default)
    pub fn build(mut self) -> Element<ButtonComponent> {
        self.el.style = match self.style {
            Some(style) => style,
            None if self.el.url.is_some() => ButtonStyle::Link,
            None if self.el.sku_id.is_some() => ButtonStyle::Premium,
            None => ButtonStyle::Secondary,
        };
        if self.label.is_some() {
            self.el.label = self.label;
        }
        Element::new(self.el, self.extras)
    }
}

/// A select: `<StringSelect>` (`<Select>`), `<UserSelect>`, `<RoleSelect>`,
/// `<MentionableSelect>`, `<ChannelSelect>`. In a `<Row>` in messages, in a `<Label>` in modals.
#[derive(Debug)]
#[must_use]
pub struct SelectBuilder<const T: u8> {
    el: SelectMenu<T>,
    options: List<SelectOption>,
    many: bool,
}

impl<const T: u8> SelectMenu<T> {
    /// A select built as its tag does
    pub fn builder() -> SelectBuilder<T> {
        SelectBuilder {
            el: SelectMenu::new(String::new(), None, None, None, None, None, None),
            options: List::default(),
            many: false,
        }
    }
}

impl<const T: u8> SelectBuilder<T> {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `String`: shown while nothing is selected; max 150 characters
        placeholder: Option<String> => el.placeholder;
        /// `i32`: fewest selections, 0-25 (default 1)
        min: Option<i32> => el.min_values;
        /// `i32`: fewest selections, 0-25 (default 1)
        min_values: Option<i32> => el.min_values;
        /// `i32`: most selections, 1-25 (default 1; a `Vec` handler parameter makes it every
        /// option)
        max: Option<i32> => el.max_values;
        /// `i32`: most selections, 1-25 (default 1; a `Vec` handler parameter makes it every
        /// option)
        max_values: Option<i32> => el.max_values;
        /// `bool`: not selectable; messages only
        disabled: Option<bool> => el.disabled;
        /// `bool`: modals only; set from the submit parameter's type
        required: Option<bool> => el.required;
    }

    /// `on:select=handler(state..)`: answered by a `#[component]` handler.
    /// `on:select:name=|ctx, values: T| ..` is an inline handler (`#[view]` only).
    pub fn on_select(self, custom_id: impl IntoCustomId) -> Self {
        self.custom_id(custom_id)
    }

    /// `String`: answered by whatever handles this `custom_id`; max 100 characters
    pub fn custom_id(mut self, custom_id: impl IntoCustomId) -> Self {
        self.el.custom_id = custom_id.into_custom_id(&mut self.options.extras);
        self
    }

    /// In a modal (required there): the input's `custom_id`, matched to an `on:submit` parameter
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.el.custom_id = name.into();
        self
    }

    /// Several selections (a `Vec` handler parameter): `max_values` becomes every option, unless
    /// set
    #[doc(hidden)]
    pub fn many(mut self, many: bool) -> Self {
        self.many = many;
        self
    }

    /// `required` from the submit parameter's type, unless set
    #[doc(hidden)]
    pub fn required_default(mut self, required: Option<bool>) -> Self {
        if self.el.required.is_none() {
            self.el.required = required;
        }
        self
    }

    /// The select
    pub fn build(mut self) -> Element<SelectMenu<T>> {
        if T == 3 {
            self.el.options = Some(self.options.items);
        }
        if self.many && self.el.max_values.is_none() {
            let all = self.el.options.as_ref().map_or(25, Vec::len).min(25);
            self.el.max_values = Some(all as i32);
        }
        Element::new(self.el, self.options.extras)
    }
}

impl SelectBuilder<3> {
    reserve!(options.items);

    /// A `#[derive(Choice)]` enum: one option per variant, before any `<Option>` children
    pub fn options<C: ModalChoice>(mut self) -> Self {
        self.options
            .items
            .extend(C::OPTIONS.iter().map(|(label, value)| {
                SelectOption::new(label.to_string(), value.to_string(), None, None, None)
            }));
        self
    }

    /// Adds an `<Option>` (or `SelectOption`s)
    pub fn child(mut self, option: impl SelectOptionChild) -> Self {
        option.add_to(&mut self.options);
        self
    }
}

macro_rules! entity_select {
    ($($n:literal),*) => {$(
        impl SelectBuilder<$n> {
            setters! {
                /// `Vec<SelectDefaultValue>`: preselected entities, `vec![SelectDefaultValue::user(id)]`
                default_values: Option<Vec<SelectDefaultValue>> => el.default_values;
            }
        }
    )*};
}

entity_select!(5, 6, 7, 8);

impl SelectBuilder<8> {
    setters! {
        /// `Vec<ChannelType>`: the kinds of channel offered
        channel_types: Option<Vec<ChannelType>> => el.channel_types;
    }
}

/// `<Option>`: in a select, `<RadioGroup>` or `<CheckboxGroup>`. Children: the label.
#[derive(Debug)]
#[must_use]
pub struct OptionBuilder {
    option: ChoiceOption,
    label: Option<String>,
}

impl SelectOption {
    /// An option built as `<Option>` does (for a select or a group)
    pub fn builder() -> OptionBuilder {
        OptionBuilder {
            option: ChoiceOption {
                label: String::new(),
                value: String::new(),
                description: None,
                emoji: None,
                default: None,
            },
            label: None,
        }
    }
}

impl OptionBuilder {
    setters! {
        /// *Required.* `impl Into<String>`: what is submitted when chosen; max 100 characters
        value: String => option.value;
        /// `String`: the label, instead of children; max 100 characters
        label: String => option.label;
        /// `String`: a second line under the label; max 100 characters
        description: Option<String> => option.description;
        /// `&str` or `PartialEmoji`: selects only
        emoji: Option<PartialEmoji> => option.emoji;
        /// `bool`: chosen initially
        default: Option<bool> => option.default;
    }

    /// Appends to the label: a string, a number, or an `Option` of one
    pub fn child(mut self, text: impl IntoText) -> Self {
        text.push_text(self.label.get_or_insert_with(String::new));
        self
    }

    /// The option
    pub fn build(mut self) -> Element<ChoiceOption> {
        if let Some(label) = self.label {
            self.option.label = label;
        }
        Element::new(self.option, Extras::default())
    }
}

/// `<Modal>`: a modal, as a [`ModalView`]; the markup's only root
#[derive(Debug)]
#[must_use]
pub struct ModalBuilder {
    data: ModalCallbackData,
    kids: Kids,
}

impl ModalView {
    /// A modal built child by child, as `<Modal>` does
    pub fn builder() -> ModalBuilder {
        ModalBuilder {
            data: ModalCallbackData {
                custom_id: String::new(),
                title: String::new(),
                components: None,
                tts: None,
                content: None,
                embeds: None,
                allowed_mentions: None,
                flags: None,
            },
            kids: Kids::default(),
        }
    }
}

impl ModalBuilder {
    setters! {
        /// *Required.* `impl Into<String>`: shown at the top; max 45 characters
        title: String => data.title;
    }

    reserve!(kids);

    /// `on:submit=handler(state..)`: submitted to a `#[modal]` handler.
    /// `on:submit:name=|ctx, field: T, ..| ..` is an inline handler (`#[view]` only).
    pub fn on_submit(self, custom_id: impl IntoCustomId) -> Self {
        self.custom_id(custom_id)
    }

    /// `String`: submitted to whatever handles this `custom_id`
    pub fn custom_id(mut self, custom_id: impl IntoCustomId) -> Self {
        self.data.custom_id = custom_id.into_custom_id(&mut self.kids.extras);
        self
    }

    /// Adds text, a `<Text>` or a `<Label>`
    pub fn child(mut self, child: impl ModalChild) -> Self {
        child.add_to(&mut self.kids);
        self
    }

    /// The modal
    pub fn build(mut self) -> ModalView {
        let (components, extras) = self.kids.finish();
        self.data.components = Some(components);
        ModalView {
            data: self.data,
            error: extras.into_inner().error,
        }
    }
}

/// `<Label>`: a title and optional description above one modal input
#[derive(Debug)]
#[must_use]
pub struct LabelBuilder {
    id: Option<i32>,
    text: String,
    description: Option<String>,
    input: Option<Component>,
    extras: Extras,
}

impl Label {
    /// A label built as `<Label>` does
    pub fn builder() -> LabelBuilder {
        LabelBuilder {
            id: None,
            text: String::new(),
            description: None,
            input: None,
            extras: Extras::default(),
        }
    }
}

impl LabelBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => id;
        /// *Required.* `impl Into<String>`: the title; max 45 characters
        text: String => text;
        /// *Required.* `impl Into<String>`: the title (the same as `text`); max 45 characters
        label: String => text;
        /// `String`: under the title; max 100 characters
        description: Option<String> => description;
    }

    /// Sets the input
    pub fn child(mut self, input: impl LabelChild) -> Self {
        input.add_to(&mut self);
        self
    }

    /// The label; without an input, a separator stands in (Discord rejects it, as `rsx!` does at
    /// compile time)
    pub fn build(self) -> Element<Label> {
        let input = self
            .input
            .unwrap_or_else(|| Component::Separator(Separator::new()));
        let mut label = Label::new(self.text, input);
        label.id = self.id;
        label.description = self.description;
        Element::new(label, self.extras)
    }
}

/// `<TextInput>`: free text, in a `<Label>` in a modal
#[derive(Debug)]
#[must_use]
pub struct TextInputBuilder {
    el: TextInput,
}

impl TextInput {
    /// A text input built as `<TextInput>` does: no built-in label (a `<Label>` gives it one)
    pub fn builder() -> TextInputBuilder {
        let mut el = TextInput::new(
            String::new(),
            TextInputStyle::Short,
            String::new(),
            None,
            None,
            None,
            None,
            None,
        );
        el.label = None;
        TextInputBuilder { el }
    }
}

macro_rules! input_name {
    () => {
        /// *Required.* A string literal: the input's `custom_id`, matched to an `on:submit`
        /// parameter
        pub fn name(mut self, name: impl Into<String>) -> Self {
            self.el.custom_id = name.into();
            self
        }

        /// `required` from the submit parameter's type, unless set
        #[doc(hidden)]
        pub fn required_default(mut self, required: Option<bool>) -> Self {
            if self.el.required.is_none() {
                self.el.required = required;
            }
            self
        }
    };
}

impl TextInputBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `TextInputStyle`: single-line or paragraph, as a value
        style: TextInputStyle => el.style;
        /// `i32`: shortest input, 0-4000
        min_len: Option<i32> => el.min_length;
        /// `i32`: shortest input, 0-4000
        min_length: Option<i32> => el.min_length;
        /// `i32`: longest input, 1-4000
        max_len: Option<i32> => el.max_length;
        /// `i32`: longest input, 1-4000
        max_length: Option<i32> => el.max_length;
        /// `String`: shown while empty; max 100 characters
        placeholder: Option<String> => el.placeholder;
        /// `String`: prefilled text; max 4000 characters
        value: Option<String> => el.value;
        /// `bool`: set from the submit parameter's type
        required: Option<bool> => el.required;
    }

    input_name!();

    /// Flag: multi-line; single-line by default
    pub fn paragraph(mut self, paragraph: bool) -> Self {
        if paragraph {
            self.el.style = TextInputStyle::Paragraph;
        }
        self
    }

    /// The text input
    pub fn build(self) -> Element<TextInput> {
        Element::new(self.el, Extras::default())
    }
}

/// `<RadioGroup>`: pick one of 2-10 options, in a `<Label>` in a modal
#[derive(Debug)]
#[must_use]
pub struct RadioGroupBuilder {
    el: RadioGroup,
    options: List<GroupOption>,
}

impl RadioGroup {
    /// A radio group built as `<RadioGroup>` does
    pub fn builder() -> RadioGroupBuilder {
        RadioGroupBuilder {
            el: RadioGroup::new(String::new(), Vec::new()),
            options: List::default(),
        }
    }
}

/// `options::<Choice>()` and `<Option>` children of a group
macro_rules! group_options {
    () => {
        reserve!(options.items);

        /// A `#[derive(Choice)]` enum: one option per variant
        pub fn options<C: ModalChoice>(mut self) -> Self {
            self.options.items.extend(
                C::OPTIONS
                    .iter()
                    .map(|(label, value)| GroupOption::new(*label, *value)),
            );
            self
        }

        /// Adds an `<Option>` (or `GroupOption`s)
        pub fn child(mut self, option: impl GroupOptionChild) -> Self {
            option.add_to(&mut self.options);
            self
        }
    };
}

impl RadioGroupBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `bool`: set from the submit parameter's type
        required: Option<bool> => el.required;
    }

    input_name!();
    group_options!();

    /// The radio group
    pub fn build(mut self) -> Element<RadioGroup> {
        self.el.options = self.options.items;
        Element::new(self.el, self.options.extras)
    }
}

/// `<CheckboxGroup>`: pick any of 1-10 options, in a `<Label>` in a modal
#[derive(Debug)]
#[must_use]
pub struct CheckboxGroupBuilder {
    el: CheckboxGroup,
    options: List<GroupOption>,
}

impl CheckboxGroup {
    /// A checkbox group built as `<CheckboxGroup>` does
    pub fn builder() -> CheckboxGroupBuilder {
        CheckboxGroupBuilder {
            el: CheckboxGroup::new(String::new(), Vec::new()),
            options: List::default(),
        }
    }
}

impl CheckboxGroupBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `u8`: fewest picked, 0-10
        min: Option<u8> => el.min_values;
        /// `u8`: fewest picked, 0-10
        min_values: Option<u8> => el.min_values;
        /// `u8`: most picked, 1-10
        max: Option<u8> => el.max_values;
        /// `u8`: most picked, 1-10
        max_values: Option<u8> => el.max_values;
        /// `bool`: set from the submit parameter's type
        required: Option<bool> => el.required;
    }

    input_name!();
    group_options!();

    /// The checkbox group
    pub fn build(mut self) -> Element<CheckboxGroup> {
        self.el.options = self.options.items;
        Element::new(self.el, self.options.extras)
    }
}

/// `<Checkbox>`: one yes/no, submitted as a `bool`, in a `<Label>` in a modal
#[derive(Debug)]
#[must_use]
pub struct CheckboxBuilder {
    el: Checkbox,
}

impl Checkbox {
    /// A checkbox built as `<Checkbox>` does
    pub fn builder() -> CheckboxBuilder {
        CheckboxBuilder {
            el: Checkbox::new(String::new()),
        }
    }
}

impl CheckboxBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `bool`: checked initially
        default: Option<bool> => el.default;
    }

    /// *Required.* A string literal: the input's `custom_id`, matched to an `on:submit` parameter
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.el.custom_id = name.into();
        self
    }

    /// The checkbox
    pub fn build(self) -> Element<Checkbox> {
        Element::new(self.el, Extras::default())
    }
}

/// `<FileUpload>`: attach files, submitted as `Attachment`s, in a `<Label>` in a modal
#[derive(Debug)]
#[must_use]
pub struct FileUploadBuilder {
    el: FileUpload,
    many: bool,
}

impl FileUpload {
    /// A file upload built as `<FileUpload>` does
    pub fn builder() -> FileUploadBuilder {
        FileUploadBuilder {
            el: FileUpload::new(String::new()),
            many: false,
        }
    }
}

impl FileUploadBuilder {
    setters! {
        /// `i32`: identifier unique within the message; Discord numbers components without one
        id: Option<i32> => el.id;
        /// `u8`: fewest files, 0-10
        min: Option<u8> => el.min_values;
        /// `u8`: fewest files, 0-10
        min_values: Option<u8> => el.min_values;
        /// `u8`: most files, 1-10 (a `Vec` parameter makes it 10)
        max: Option<u8> => el.max_values;
        /// `u8`: most files, 1-10 (a `Vec` parameter makes it 10)
        max_values: Option<u8> => el.max_values;
        /// `Vec<String>`: up to 10 of `"image"`, `"video"`, `"audio"`, or extensions like `".pdf"`
        file_types: Option<Vec<String>> => el.file_types;
        /// `bool`: set from the submit parameter's type
        required: Option<bool> => el.required;
    }

    input_name!();

    /// Several files (a `Vec` submit parameter): up to 10, unless `max` is set
    #[doc(hidden)]
    pub fn many(mut self, many: bool) -> Self {
        self.many = many;
        self
    }

    /// The file upload
    pub fn build(mut self) -> Element<FileUpload> {
        if self.many && self.el.max_values.is_none() {
            self.el.max_values = Some(10);
        }
        Element::new(self.el, Extras::default())
    }
}

/// Every built-in tag, by its name in markup (aliases included). `rsx!` resolves a tag it does
/// not know through these, so a name being typed completes from the built-in tags and the
/// sub-Views in scope alike.
#[doc(hidden)]
pub mod elements {
    pub use super::super::ModalView as Modal;
    pub use crate::types::{
        ActionRow, ActionRow as Row, ButtonComponent as Button, ChannelSelect, Checkbox,
        CheckboxGroup, Container, FileComponent as File, FileUpload, Label, MediaGallery,
        MediaGallery as Gallery, MediaGalleryItem, MediaGalleryItem as Item, MentionableSelect,
        RadioGroup, RoleSelect, Section, SelectOption as Option, Separator, StringSelect,
        StringSelect as Select, TextDisplay, TextDisplay as Text, TextInput, Thumbnail, UserSelect,
    };
}
