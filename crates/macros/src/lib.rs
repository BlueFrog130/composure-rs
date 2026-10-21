//! Procedural macros for [Composure](https://docs.rs/composure), the Discord Interactions
//! framework. Use them through `composure` with its `macros` feature, which re-exports them:
//! `composure::{app, command, group, component, modal, autocomplete, view, rsx, Choice, Modal}`.
//!
//! | Macro | |
//! |---|---|
//! | [`#[app]`](macro@app) | Collects Commands, Groups, handlers, Views and other Apps into the handler an Adapter serves |
//! | [`#[command]`](macro@command) | A Command from a function: Options from its parameters, the description from its doc comment |
//! | [`#[group]`](macro@group) | A Command whose subcommands are other Commands |
//! | [`#[autocomplete]`](macro@autocomplete) | Suggestions for an Option while the user types |
//! | [`#[component]`](macro@component) | Answers buttons and selects by `custom_id` prefix, with state in the `custom_id` |
//! | [`#[modal]`](macro@modal) | Answers modal submits by `custom_id` prefix |
//! | [`#[derive(Modal)]`](derive@Modal) | A form shown as a modal, read back typed |
//! | [`#[derive(Choice)]`](derive@Choice) | An enum of fixed Choices |
//! | [`rsx!`](macro@rsx) | Markup for message and modal layouts |
//! | [`#[view]`](macro@view) | A layout rendered from Props, with inline handlers |
//!
//! The [Composure guide](https://bluefrog130.github.io/composure-rs/) explains
//! each in depth; [Views](https://bluefrog130.github.io/composure-rs/views.html#tag-reference)
//! lists every `rsx!` tag and attribute.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use proc_macro::TokenStream;

mod app;
mod autocomplete;
mod choice;
mod command;
mod group;
mod handler;
mod meta;
mod modal;
mod view;

/// Makes a unit struct an App: the Interaction Handler routing to the Commands, Groups, Component
/// and Modal Handlers, Views and other Apps listed in `handlers(..)`, in any mix and order.
///
/// ```ignore
/// /// Everything the bot serves
/// #[app(handlers(ping, echo, Admin, counter_click, on_feedback, Card, Moderation),
///       events = MyEvents, errors = render_error)]
/// pub struct Bot;
/// ```
///
/// - Implements `InteractionHandler` (for every State all listed items accept), `EventHandler`
///   and `CommandSetDefinition` (for Sync); `Bot.with_fallback(handler)` sends what nothing
///   listed handles (components, modals, autocomplete, commands) to `handler`
/// - Apps nest: a module can declare its own `#[app]` next to its handlers and be listed in the
///   root one
/// - `events = MyEvents` (a unit struct implementing `EventHandler`): handles Webhook Events;
///   at most one App in a tree sets it
/// - `errors = render_error` (a `fn(&HandlerError) -> InteractionResponse`, an `ErrorRenderer`):
///   the response handler errors are answered with, here and from deferred work, instead of
///   `HandlerError::response`'s ephemeral text; at most one App in a tree sets it, and the
///   root uses it for the whole tree
///
/// Two Commands with the same name and type, two handlers with the same `custom_id` prefix, more
/// Commands than Discord allows in one Scope, a placed sub-View with handlers that is not
/// listed, or two Apps setting `events` (or `errors`) fail to compile.
#[proc_macro_attribute]
pub fn app(args: TokenStream, item: TokenStream) -> TokenStream {
    app::expand(args.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Defines a Command from a function. Parameters after an optional leading `ctx: Ctx` become
/// Options; the doc comment becomes the description.
///
/// ```ignore
/// /// Ban a member
/// #[command(scope = "dev", permissions = Permissions::BanMembers)]
/// async fn ban(
///     ctx: Ctx<Env>,
///     #[option("Who to ban")] user: &User,
///     #[option("Days of messages to delete", min = 0, max = 7)] days: Option<u8>,
/// ) -> Result<Ephemeral<String>, HandlerError> { ... }
/// ```
///
/// - `ctx: Ctx` works with any State, `ctx: Ctx<T>` only with State `T`, and `ctx: Ctx<impl
///   Bound>` (or `fn f<S: Bound>(ctx: Ctx<S>)`) with any State meeting the bound
/// - `Option<T>` parameters are not required; `&str`, `&User`, `&Role`, `&PartialChannel`,
///   `&Attachment`, and `&Message` borrow from the Interaction instead of copying
/// - Settings: `name`, `desc`, `scope`, `permissions`, `contexts(...)`,
///   `integration_types(...)`, `nsfw`, and `user` / `message` for context-menu commands, which
///   take one target parameter (`User`, `ResolvedMember`, or `Message`, owned or borrowed)
/// - `#[option("Description", ...)]` then `name`, `min`, `max`, `min_len`, `max_len`,
///   `channel_types(...)`, `autocomplete = handler` (an `#[autocomplete]` function)
///
/// The function stays callable as written; a same-named empty struct is added for
/// `#[app]` and Groups to refer to.
#[proc_macro_attribute]
pub fn command(args: TokenStream, item: TokenStream) -> TokenStream {
    command::expand(args.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Makes a unit struct a Group that routes to its subcommands. Groups can contain Groups one
/// level deep.
///
/// ```ignore
/// /// Moderation tools
/// #[group(subcommands(ban, kick, Roles))]
/// struct Admin;
/// ```
///
/// The name defaults to the struct name in `snake_case`. Settings are the same as `#[command]`
/// minus `user` / `message`.
#[proc_macro_attribute]
pub fn group(args: TokenStream, item: TokenStream) -> TokenStream {
    group::expand(args.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Defines a Component Handler from a function: it answers components whose `custom_id` starts
/// with the prefix. Parameters after an optional leading `ctx: ComponentCtx` are state carried in
/// the `custom_id` (owned, serde-serializable), built with the generated `name::custom_id(..)`.
///
/// ```ignore
/// #[component("vote")]
/// fn vote(ctx: ComponentCtx, poll: u32, up: bool) -> Update<String> { ... }
///
/// // In a Command's response:
/// Component::new_button(ButtonStyle::Success, Some("Up".into()), None,
///     Some(vote::custom_id(poll, true)?), None, None)
/// ```
///
/// The return value answers like a Command's: a new message by default, `Update(..)` to edit the
/// message the component is on, `ctx.defer_update(..)` to do that later. List the handler in an
/// `#[app]`.
#[proc_macro_attribute]
pub fn component(args: TokenStream, item: TokenStream) -> TokenStream {
    handler::expand(handler::Kind::Component, args.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Defines a Modal Handler from a function: it answers submits of modals whose `custom_id`
/// starts with the prefix. After an optional leading `ctx: ModalCtx` comes the form (a
/// `#[derive(Modal)]` type), then state carried in the `custom_id`. `name::show(..)` is the
/// modal response that opens the form.
///
/// ```ignore
/// #[modal("rename")]
/// fn rename(ctx: ModalCtx, form: Rename, item: u32) -> Ephemeral<String> { ... }
///
/// #[command]
/// fn edit(#[option("Item")] item: u32) -> Result<InteractionResponse, CustomIdError> {
///     rename::show(item)
/// }
/// ```
///
/// List the handler in an `#[app]`.
#[proc_macro_attribute]
pub fn modal(args: TokenStream, item: TokenStream) -> TokenStream {
    handler::expand(handler::Kind::Modal, args.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// A form shown as a modal: each named field is one input inside a Label.
///
/// ```ignore
/// #[derive(Modal)]
/// #[modal(title = "Report a bug", text = "Thanks for helping!")]
/// struct BugReport {
///     #[input(max_len = 100)]
///     summary: String,
///     #[input(paragraph, label = "What happened?")]
///     details: Option<String>,
///     #[radio]
///     severity: Severity,               // a #[derive(Choice)] enum supplies the options
///     #[checkboxes(option("Windows", "win"), option("macOS", "mac"), option("Linux", "linux"))]
///     platforms: Vec<String>,
///     #[file_upload(max = 3, file_types("image"))]
///     screenshots: Vec<Attachment>,
///     #[checkbox(label = "Contact me")]
///     contact: bool,
/// }
/// ```
///
/// The attribute picks the input (default `#[input]`): `input`, `select`, `user_select`,
/// `role_select`, `mentionable_select`, `channel_select`, `radio`, `checkboxes`, `checkbox`,
/// `file_upload`. All take `label` (defaults to the field name), `description`, and `id`
/// (defaults to the field name); then
/// - `input`: `paragraph`, `min_len`, `max_len`, `placeholder`, `value`
/// - `select`: `placeholder`, `min`, `max`, `option("Label", "value")`...
/// - `user_select`, `role_select`, `mentionable_select`: `placeholder`, `min`, `max`
/// - `channel_select`: those and `channel_types(...)`
/// - `radio`: `option(...)`...; `checkboxes`: `min`, `max`, `option(...)`...
/// - `checkbox`: `default`; `file_upload`: `min`, `max`, `file_types("image", ".pdf")`
///
/// Without `option(...)`, a `#[derive(Choice)]` field type supplies the options. The type
/// decides required-ness (`Option` and `Vec` are optional, `Vec` allows several selections) and
/// how the value is read: `String`, `bool`, `Snowflake`, `User`, `Role`, `PartialChannel`,
/// `Attachment`, or a Choice enum, each also in `Option` or `Vec`; a mismatch with the input (a
/// `bool` text input, a `Vec` radio group) fails to compile. On the struct:
/// `#[modal(title = "...", text = "markdown")]`, `text` repeatable; at most 5 fields and texts.
#[proc_macro_derive(
    Modal,
    attributes(
        modal,
        input,
        select,
        user_select,
        role_select,
        mentionable_select,
        channel_select,
        radio,
        checkboxes,
        checkbox,
        file_upload
    )
)]
pub fn derive_modal(item: TokenStream) -> TokenStream {
    modal::expand(item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Defines an Autocomplete Handler from a function taking an optional leading `ctx: Ctx` and
/// what the user has typed so far as `input: &str`. Attach it to a string, integer, or number
/// Option with `#[option("...", autocomplete = name)]`.
///
/// ```ignore
/// #[autocomplete]
/// fn fruits(input: &str) -> Vec<&'static str> {
///     FRUITS.iter().copied().filter(|f| f.starts_with(input)).collect()
/// }
///
/// /// Pick a fruit
/// #[command]
/// fn fruit(#[option("Fruit", autocomplete = fruits)] name: &str) -> String { ... }
/// ```
///
/// Return a `Vec` of strings, numbers, `(name, value)` pairs, or
/// `ApplicationCommandOptionChoice`s (at most 25 are sent), or a `Result` of one.
#[proc_macro_attribute]
pub fn autocomplete(args: TokenStream, item: TokenStream) -> TokenStream {
    autocomplete::expand(args.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Makes a fieldless enum (1-25 variants) usable as an Option restricted to its variants, and as
/// the options of selects, radio groups and checkbox groups.
///
/// ```ignore
/// #[derive(Choice)]
/// enum Flavor {
///     Vanilla,
///     #[choice(name = "Mint chip", value = "mint")]
///     MintChip,
/// }
/// ```
///
/// - `#[choice(name = "...")]`: the name shown to users (1-100 characters); the variant's
///   identifier by default
/// - `#[choice(value = "...")]`: what Discord sends; the identifier by default. If every variant
///   sets an integer `#[choice(value = N)]`, the Option is an integer Option.
///
/// Adds `name()` and `value()` for the chosen variant.
#[proc_macro_derive(Choice, attributes(choice))]
pub fn derive_choice(item: TokenStream) -> TokenStream {
    choice::expand(item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Markup for a message or modal layout, as JSX: built-in tags are Discord's components
/// (`<Container>`, `<Section>`, `<Text>`, `<Thumbnail>`, `<Gallery>`/`<Item>`, `<File>`,
/// `<Separator>`, `<Row>`, `<Button>`, `<StringSelect>`/`<Option>`, `<UserSelect>`, ...; in a
/// `<Modal>`: `<Label>`, `<TextInput>`, `<RadioGroup>`, `<CheckboxGroup>`, `<Checkbox>`,
/// `<FileUpload>`), PascalCase tags are `#[view]`s. Evaluates to a `View`, or a `ModalView` for a
/// `<Modal>` root.
///
/// ```ignore
/// rsx! {
///     <Container accent_color=0x5865F2>
///         "## " {title.as_str()}
///         <Section>
///             "Adjacent text and `{..}` join into one text display"
///             <Thumbnail url=avatar/>
///         </Section>
///         <Row>
///             <Button primary on:click=vote(poll, true)>"Up"</Button>
///             <Button url="https://discord.com">"Docs"</Button>
///         </Row>
///     </Container>
/// }
/// ```
///
/// Attributes set the component's fields (`accent_color`, `spoiler`, `disabled`, `id`, ...);
/// a bare attribute is `true`. `{..}` children are text, components, Views, Props, or `Option`s,
/// `Vec`s and iterators of them. `on:click=handler(state..)` / `on:select=..` / `on:submit=..`
/// point at a `#[component]` or `#[modal]` handler; inline handlers (`on:click:name=|ctx| ..`)
/// need the markup to be in a `#[view]`. A `<Section>`'s `<Button>` or `<Thumbnail>` child is
/// its accessory. Wrap attribute values that use `<..>` in braces.
///
/// Every error in the markup is reported, not just the first. A tag with an error is left out
/// and the rest still expands (to `View::default()` when nothing parsed), so `{..}` blocks keep
/// type-checking and completing in incomplete markup.
///
/// | Tag | Builds | Attributes (every tag but `<Modal>`, `<Item>` and `<Option>` also takes `id`) |
/// |---|---|---|
/// | `<Container>` | `Container` | `accent_color`, `spoiler` |
/// | `<Section>` | `Section` | `accessory` (or a `<Button>` / `<Thumbnail>` child) |
/// | `<Text>` (`<TextDisplay>`) | `TextDisplay` | `content` (or text children) |
/// | `<Separator>` | `Separator` | `small` / `large`, `spacing`, `divider` |
/// | `<Row>` (`<ActionRow>`) | `ActionRow` | |
/// | `<Thumbnail>` | `Thumbnail` | `url` (required), `description`, `spoiler` |
/// | `<Gallery>` (`<MediaGallery>`) | `MediaGallery` | `<Item url=..>` children: `description`, `spoiler` |
/// | `<File>` | `FileComponent` | `url` (required, `attachment://..`), `spoiler` |
/// | `<Button>` | `ButtonComponent` | `primary` / `secondary` / `success` / `danger`, `style`, `on:click`, `custom_id`, `url`, `sku_id`, `label`, `emoji`, `disabled` |
/// | `<StringSelect>` (`<Select>`) | `StringSelect` | `on:select`, `custom_id`, `name` (modals), `options` (a Choice enum), `placeholder`, `min`, `max`, `disabled`, `required` |
/// | `<UserSelect>`, `<RoleSelect>`, `<MentionableSelect>`, `<ChannelSelect>` | the entity selects | as `<StringSelect>` without `options`, plus `default_values`, `channel_types` |
/// | `<Option>` | `SelectOption` / `GroupOption` | `value` (required), `label` (or text children), `description`, `emoji`, `default` |
/// | `<Modal>` | `ModalView` | `title` (required), `on:submit` or `custom_id` |
/// | `<Label>` | `Label` | `text` (required), `description` |
/// | `<TextInput>` | `TextInput` | `name`, `paragraph`, `style`, `min_len`, `max_len`, `placeholder`, `value`, `required` |
/// | `<RadioGroup>` | `RadioGroup` | `name`, `options`, `required` |
/// | `<CheckboxGroup>` | `CheckboxGroup` | `name`, `options`, `min`, `max`, `required` |
/// | `<Checkbox>` | `Checkbox` | `name`, `default` |
/// | `<FileUpload>` | `FileUpload` | `name`, `min`, `max`, `file_types`, `required` |
#[proc_macro]
pub fn rsx(input: TokenStream) -> TokenStream {
    view::expand_macro(input.into()).into()
}

/// Defines a View: a layout rendered from its Props (the parameters, owned values), whose
/// `rsx!`s may hold inline handlers.
///
/// ```ignore
/// #[view]
/// pub fn Counter(count: u32, #[prop(default = 1)] step: u32, note: Option<String>) -> View {
///     rsx! {
///         <Text>"Clicked " {count} " times"</Text>
///         <Row>
///             <Button primary on:click:add=move |_| Counter { count: count + step, step, note }>
///                 "+" {step}
///             </Button>
///         </Row>
///     }
/// }
/// ```
///
/// Adds a same-named Props struct: return it (or the View) from a Command to send the View, from
/// an inline handler to re-render the message, or place it in markup as `<Counter count=0/>`
/// (`Option` and `#[prop(default = ..)]` Props may be left out; a `children: View` Prop takes the
/// tag's children). An inline handler `on:click:name=move |ctx| ..` (`on:select:name=|ctx,
/// values: Vec<Choice>| ..`, `on:submit:name=|ctx, field: Type, ..| ..` on a `<Modal>`) becomes a
/// handler routed by `custom_id` `prefix:name:state`; the Props it names are its state. The
/// prefix defaults to the name in `snake_case` (`#[view(prefix = "c")]`). List Views with
/// handlers in an `#[app]`.
#[proc_macro_attribute]
pub fn view(args: TokenStream, item: TokenStream) -> TokenStream {
    view::expand_attr(args.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
