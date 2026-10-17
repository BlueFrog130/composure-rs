//! Macros for defining Composure Commands and handlers. Use them through `composure` with the
//! `macros` feature: `composure::{command, group, component, modal, Choice, Modal}`.

use proc_macro::TokenStream;

mod autocomplete;
mod choice;
mod command;
mod group;
mod handler;
mod meta;
mod modal;

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
/// `commands!` and Groups to refer to.
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
/// message the component is on, `ctx.defer_update(..)` to do that later. List the handler in
/// `commands!` under `components = [...]`.
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
/// List the handler in `commands!` under `modals = [...]`.
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

/// Makes a fieldless enum usable as an Option restricted to its variants. Values are the
/// variant names unless every variant sets an integer `#[choice(value = N)]`; `#[choice(name =
/// "...")]` sets the name shown to users. Adds `name()` and `value()` for the chosen variant.
#[proc_macro_derive(Choice, attributes(choice))]
pub fn derive_choice(item: TokenStream) -> TokenStream {
    choice::expand(item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
