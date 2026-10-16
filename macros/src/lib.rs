//! Macros for defining Composure Commands. Use them through `composure` with the `macros`
//! feature: `composure::{command, group, Choice}`.

use proc_macro::TokenStream;

mod choice;
mod command;
mod group;
mod meta;

/// Defines a Command from a function. Parameters after an optional leading `ctx: Ctx` become
/// Options; the doc comment becomes the description.
///
/// ```ignore
/// /// Ban a member
/// #[command(scope = "dev", permissions = Permissions::BanMembers)]
/// async fn ban(
///     ctx: Ctx<Env>,
///     #[option(desc = "Who to ban")] user: User,
///     #[option(desc = "Days of messages to delete", min = 0, max = 7)] days: Option<u8>,
/// ) -> Result<String, worker::Error> { ... }
/// ```
///
/// - `ctx: Ctx` works with any State; `ctx: Ctx<T>` only with State `T`
/// - `Option<T>` parameters are not required
/// - Settings: `name`, `desc`, `scope`, `permissions`, `contexts(...)`,
///   `integration_types(...)`, `nsfw`, and `user` / `message` for context-menu commands, which
///   take one target parameter (`User`, `ResolvedMember`, or `Message`)
/// - `#[option(...)]`: `desc` (required), `name`, `min`, `max`, `min_len`, `max_len`,
///   `channel_types(...)`
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

/// Makes a fieldless enum usable as an Option restricted to its variants. Values are the
/// variant names unless every variant sets an integer `#[choice(value = N)]`; `#[choice(name =
/// "...")]` sets the name shown to users.
#[proc_macro_derive(Choice, attributes(choice))]
pub fn derive_choice(item: TokenStream) -> TokenStream {
    choice::expand(item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
