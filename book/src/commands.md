# Commands

A Command is defined once, as a function with [`#[command]`](https://docs.rs/composure/latest/composure/attr.command.html): the function is
the handler, and its signature is the
[Command Definition](https://discord.com/developers/docs/interactions/application-commands)
synced to Discord.

```rust
use composure::{command, types::User};

/// Give someone a cookie
#[command]
fn cookie(
    #[option("Who gets it")] user: &User,
    #[option("How many", min = 1, max = 12)] count: Option<u8>,
) -> String {
    format!("{} gets {} 🍪", user.display_name(), count.unwrap_or(1))
}
# fn main() {}
```

- The **name** is the function's (`cookie`); the **description** its doc comment.
- Each parameter with `#[option("description", ..)]` is an **Option**; its Rust type decides the
  Option's type and how the value is read.
- The **return value** answers the Interaction (see [Responses](responses.md)).
- The function stays an ordinary function you can call; the macro adds a same-named empty struct
  (`cookie`) that implements the Command for `#[app]` and `#[group]` to list.

Functions may be `async`. Everything is checked at compile time: names, description lengths,
bounds against the parameter type, at most 25 Options, required Options before optional ones.

## The context

An optional first parameter `ctx: Ctx` gives the handler the Interaction and the Adapter's
State:

```rust
use composure::{command, command::Ctx};

/// Say hello
#[command]
fn hello(ctx: Ctx) -> String {
    let name = match (ctx.member(), ctx.user()) {
        (Some(member), _) => member.display_name(), // server nickname in guilds
        (None, Some(user)) => user.display_name(),
        (None, None) => "stranger",
    };
    format!("Hello, {name}!")
}
# fn main() {}
```

| [`Ctx`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html) method | |
|---|---|
| [`user()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.user) | The invoking user, in guilds and DMs |
| [`member()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.member) | The invoking member, in guilds only |
| [`guild_id()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.guild_id), [`channel_id()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.channel_id) | Where it was invoked |
| [`locale()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.locale) | The user's language |
| [`entitlements()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.entitlements), [`is_entitled(sku)`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.is_entitled) | Premium SKUs the user or guild has |
| [`theme()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.theme) | The App's [Theme](theming.md) |
| [`interaction()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.interaction), [`data()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.data) | Everything Discord sent |
| [`state()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.state) | The Adapter's State |
| [`client()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.client), [`bot()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.bot), [`defer(..)`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.defer) | REST and Background Work, with a [`Platform`](https://docs.rs/composure/latest/composure/command/trait.Platform.html) State ([REST and deferring](deferring.md)) |

The State type parameter says which Adapters can serve the Command:

| Parameter | Works with |
|---|---|
| `ctx: Ctx` | any State |
| `ctx: Ctx<impl Platform>` (or `fn f<S: Platform>(ctx: Ctx<S>)`) | any State meeting the bound; prefer this for REST and deferring |
| `ctx: Ctx<impl MyTrait>` | any State implementing your trait (see [Platforms](platforms.md#platform-specific-commands)) |
| `ctx: Ctx<Worker>`, `ctx: Ctx<Host<MyData>>` | that one Adapter's State |

An App can then only be served with a State every Command it lists accepts.

## Options

| Parameter type | Option type | Settings |
|---|---|---|
| `&str`, `String` | string | `min_len`, `max_len` (0-6000), `autocomplete` |
| `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32` | integer | `min`, `max` (inside the type's range), `autocomplete` |
| `f64` | number | `min`, `max`, `autocomplete` |
| `bool` | boolean | |
| `&User`, `User` | user | |
| [`ResolvedMember`](https://docs.rs/composure/latest/composure/command/struct.ResolvedMember.html) | user, with their guild membership (fails outside guilds) | |
| `&PartialChannel`, `PartialChannel` | channel | `channel_types(GuildText, ..)` |
| `&Role`, `Role` | role | |
| [`UserOrRole`](https://docs.rs/composure/latest/composure/command/enum.UserOrRole.html) | mentionable | |
| `&Attachment`, `Attachment` | attachment | |
| a [`#[derive(Choice)]`](#choices) enum | string or integer, restricted to its variants | |
| `Option<T>` | `T`, not required | as `T` |

Borrowed types (`&str`, `&User`, ...) read straight from the Interaction without copying; owned
ones clone. Integer types narrower than `i64` bound the Option to their range automatically,
and explicit `min`/`max` must stay inside it.

`#[option]` takes the description first, then any of:

| Setting | |
|---|---|
| `name = "..."` | The Option's name, instead of the parameter's (1-32 lowercase letters, digits, `-`, `_`) |
| `min = N`, `max = N` | Bounds of integer and number Options |
| `min_len = N`, `max_len = N` | Length bounds of string Options |
| `channel_types(GuildText, GuildVoice, ..)` | [`ChannelType`](https://docs.rs/composure/latest/composure/types/enum.ChannelType.html)s a channel Option offers |
| `autocomplete = handler` | Suggestions while typing, from an [`#[autocomplete]`](#autocomplete) function |

```rust
use composure::{command, types::{PartialChannel, Role}};

/// Post a role menu
#[command]
fn rolemenu(
    #[option("Where to post it", channel_types(GuildText, GuildAnnouncement))] channel: &PartialChannel,
    #[option("The role to offer")] role: &Role,
    #[option("Menu title", name = "title", max_len = 80)] heading: Option<&str>,
) -> String {
    format!("{} in <#{}> for {}", heading.unwrap_or("Roles"), channel.id, role.name)
}
# fn main() {}
```

When Discord sends a value that cannot be read (a stale definition, a missing resolved user),
the handler is not called and the user gets an error message; see
[Responses](responses.md#errors).

## Command settings

`#[command(..)]` (and `#[group(..)]`) accept:

| Setting | |
|---|---|
| `name = "..."` | The Command's name, instead of the function's |
| `desc = "..."` | The description, instead of the doc comment (1-100 characters) |
| `scope = "..."` | Sync to the guilds of this [Scope](#scopes) instead of globally |
| `permissions = ..` | Default member permissions: `Permissions::BanMembers`, `Permissions::ManageMessages \| Permissions::KickMembers` ([`Permissions`](https://docs.rs/composure/latest/composure/types/struct.Permissions.html) is in scope) |
| `contexts(Guild, BotDm, PrivateChannel)` | Where it can be used ([`InteractionContextType`](https://docs.rs/composure/latest/composure/types/enum.InteractionContextType.html)) |
| `integration_types(GuildInstall, UserInstall)` | Which installs offer it ([`ApplicationIntegrationType`](https://docs.rs/composure/latest/composure/types/enum.ApplicationIntegrationType.html)) |
| `nsfw` | Age-restricted |
| `user`, `message` | A [context menu](#context-menus) Command |

```rust
use composure::command;

/// Say hi from anywhere: servers, DMs, and group chats, installed to servers or to users
#[command(
    name = "hi",
    contexts(Guild, BotDm, PrivateChannel),
    integration_types(GuildInstall, UserInstall),
)]
fn say_hi() -> &'static str {
    "Hi!"
}
# fn main() {}
```

## Choices

`#[derive(Choice)]` on a fieldless enum restricts an Option to its variants. Discord shows
them as a dropdown.

```rust
use composure::{Choice, command};

#[derive(Choice)]
pub enum Flavor {
    Vanilla,
    Chocolate,
    #[choice(name = "Mint chip", value = "mint")]
    MintChip,
}

/// Order ice cream
#[command(name = "ice-cream")]
fn ice_cream(#[option("Flavor")] flavor: Flavor) -> String {
    format!("One {}, coming up", flavor.name())
}
# fn main() {}
```

- A variant's **name** (shown to the user) is its identifier unless `#[choice(name = "..")]`
  sets it (1-100 characters).
- Its **value** (what Discord sends) is the identifier unless `#[choice(value = "..")]` sets it.
  If every variant sets an integer `#[choice(value = N)]`, the Option is an integer Option.
- `flavor.name()` and `flavor.value()` give them back.
- 1-25 variants.

The same enums supply the options of [selects and radio groups](components.md).

## Autocomplete

An [`#[autocomplete]`](https://docs.rs/composure/latest/composure/attr.autocomplete.html) function suggests values for a string, integer or
number Option while the user types. It takes an optional `ctx: Ctx` and what has been typed so
far as `input: &str`:

```rust
use composure::{autocomplete, command};

const FRUITS: [&str; 6] = ["apple", "apricot", "banana", "cherry", "grape", "mango"];

/// Fruits starting with what the user typed
#[autocomplete]
fn fruit_names(input: &str) -> Vec<&'static str> {
    let input = input.to_lowercase();
    FRUITS.into_iter().filter(|fruit| fruit.starts_with(&input)).collect()
}

/// Pick a fruit
#[command]
fn fruit(#[option("Fruit", autocomplete = fruit_names)] name: &str) -> String {
    format!("You picked {name}")
}
# fn main() {}
```

It returns a `Vec` of suggestions, or a `Result` of one: strings (shown and sent as is), numbers,
`(name, value)` pairs, or [`ApplicationCommandOptionChoice`](https://docs.rs/composure/latest/composure/types/struct.ApplicationCommandOptionChoice.html)s
([`IntoChoice`](https://docs.rs/composure/latest/composure/command/trait.IntoChoice.html)). Discord shows at most 25; the rest are dropped.
Autocomplete handlers can be `async` and use REST like Commands, but must answer within the
same 3 seconds, and cannot defer.

Suggestions are only suggestions: the user can still send any value, so validate it in the
Command. Options with Choices cannot autocomplete.

## Groups

[`#[group]`](https://docs.rs/composure/latest/composure/attr.group.html) on a unit struct makes a Command whose subcommands are other Commands
(or Groups): `/mod ban`, `/mod kick`.

```rust
use composure::{command, group, types::User};

/// Ban a member
#[command]
fn ban(#[option("Who")] user: &User) -> String {
    format!("Banned {}", user.display_name())
}

/// Kick a member
#[command]
fn kick(#[option("Who")] user: &User) -> String {
    format!("Kicked {}", user.display_name())
}

/// Moderation tools
#[group(subcommands(ban, kick), permissions = Permissions::BanMembers)]
pub struct Mod;
# fn main() {}
```

- The name defaults to the struct's in `snake_case`; the description to its doc comment.
- Groups take the same settings as Commands, except `user`/`message`. Settings like
  `permissions`, `contexts` and `scope` belong to the top level: Discord ignores them on
  subcommands, and `scope` on a subcommand is a compile error.
- Groups nest one level: a Group may list Groups whose subcommands are Commands
  (`/settings privacy dms`).
- List the Group (not its subcommands) in the `#[app]`. A subcommand can still be listed in an
  App on its own as a top-level Command.

## Context menus

`#[command(user)]` and `#[command(message)]` make Commands shown in the right-click menu of a
user or message. They take one target parameter instead of Options, and their names may
contain spaces and capitals:

| Kind | Target parameter |
|---|---|
| `user` | `&User`, `User`, or [`ResolvedMember`](https://docs.rs/composure/latest/composure/command/struct.ResolvedMember.html) |
| `message` | `&Message`, `Message` |

```rust
use composure::{command, types::{Message, User}};

/// Shows a user's id
#[command(user, name = "Copy id")]
fn copy_id(target: &User) -> String {
    format!("`{}`", target.id)
}

/// Counts the words in a message
#[command(message, name = "Word count")]
fn word_count(target: &Message) -> String {
    format!("{} words", target.content.split_whitespace().count())
}
# fn main() {}
```

A doc comment is still allowed, but Discord does not show descriptions for context menus.

## Scopes

Commands are global by default. `scope = "dev"` puts a Command (or Group) in a named Scope
instead, and Sync sends it to the guilds configured for that Scope (`COMPOSURE_SCOPE_DEV=<guild
ids>`), which is how you keep admin tools to your own server or test Commands before releasing
them. See [Syncing Commands](getting_started.md#syncing-commands).

Discord allows 100 chat-input, 15 user and 15 message Commands per Scope; an App exceeding that,
or listing two Commands with the same name and type, fails to compile.

## Listing Commands

A Command does nothing until an [`#[app]`](https://docs.rs/composure/latest/composure/attr.app.html) lists it; see
[Getting started](getting_started.md#apps). The macros implement these traits, should
you need to write one by hand: [`CommandDefinition`](https://docs.rs/composure/latest/composure/command/trait.CommandDefinition.html) and
[`SubcommandDefinition`](https://docs.rs/composure/latest/composure/command/trait.SubcommandDefinition.html) (the definition),
[`Command`](https://docs.rs/composure/latest/composure/command/trait.Command.html) (the handler), and
[`RouteDefinition`](https://docs.rs/composure/latest/composure/command/trait.RouteDefinition.html) / [`Route`](https://docs.rs/composure/latest/composure/command/trait.Route.html) (what an
App lists). [`OptionType`](https://docs.rs/composure/latest/composure/command/trait.OptionType.html) and
[`CommandTarget`](https://docs.rs/composure/latest/composure/command/trait.CommandTarget.html) make your own types usable as parameters.
