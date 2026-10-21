# Responses

What a handler returns is how it answers. Commands, component and modal handlers, and inline
View handlers all answer the same way: return anything implementing
[`IntoResponse`](https://docs.rs/composure/latest/composure/command/trait.IntoResponse.html), or a `Result` of one.

```rust
use composure::{command, command::Ephemeral, types::Embed};

/// Plain text
#[command]
fn ping() -> &'static str {
    "Pong!"
}

/// Only you can see this
#[command]
fn secret() -> Ephemeral<String> {
    Ephemeral(format!("The answer is {}", 6 * 7))
}

/// An embed
#[command]
fn info() -> Embed {
    Embed::new().with_title("Composure").with_description("Interactions only")
}
# fn main() {}
```

Discord gives a bot **3 seconds** to answer an Interaction. Work that may take longer answers
with a Deferred Response instead and finishes in the background; see
[REST and deferring](deferring.md).

## What can be returned

| Type | Answer |
|---|---|
| `String`, `&str` | A message with that content |
| [`Embed`](https://docs.rs/composure/latest/composure/types/struct.Embed.html) | A message with one embed |
| [`Container`](https://docs.rs/composure/latest/composure/types/struct.Container.html) | A Components V2 message of one container |
| [`View`](https://docs.rs/composure/latest/composure/command/struct.View.html), a `#[view]`'s Props | A Components V2 message ([Views](views.md)) |
| [`ModalView`](https://docs.rs/composure/latest/composure/command/struct.ModalView.html), a modal `#[view]`'s Props | Opens a modal |
| [`InteractionResponse`](https://docs.rs/composure/latest/composure/types/enum.InteractionResponse.html) | Exactly that; anything Discord accepts |
| [`Deferred`](https://docs.rs/composure/latest/composure/command/struct.Deferred.html) | "Thinking...", then the result of Background Work ([`ctx.defer`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.defer)) |
| [`DeferredUpdate`](https://docs.rs/composure/latest/composure/command/struct.DeferredUpdate.html) | Nothing visible yet, then an edit of the component's message ([`ctx.defer_update`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.defer_update)) |
| [`Ephemeral<T>`](https://docs.rs/composure/latest/composure/command/struct.Ephemeral.html) | `T`, visible only to the user who invoked it |
| [`Update<T>`](https://docs.rs/composure/latest/composure/command/struct.Update.html) | `T` as an edit of the message the component is on (components and modals opened from one) |
| [`Unthemed<T>`](https://docs.rs/composure/latest/composure/command/struct.Unthemed.html) | `T` without the App's [Theme](theming.md) filled in |
| `Result<T, E>` with `E: Display` | `T`, or an [error message](#errors) |

The wrappers compose: `Ephemeral(Unthemed(embed))`, `Update(view)`,
`Ephemeral(ctx.defer(..))`, `Result<Ephemeral<String>, HandlerError>`.

`Ephemeral` on a deferral makes the "thinking" message ephemeral, and the final message with
it: ephemerality is decided when the Interaction is first answered and cannot change later.

## Building messages by hand

[`InteractionResponse`](https://docs.rs/composure/latest/composure/types/enum.InteractionResponse.html) is Discord's
[interaction response](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-response-object),
with constructors for the common shapes:

| Constructor | |
|---|---|
| [`respond_with_message(content)`](https://docs.rs/composure/latest/composure/types/enum.InteractionResponse.html#method.respond_with_message) | Text |
| [`respond_with_ephemeral_message(content)`](https://docs.rs/composure/latest/composure/types/enum.InteractionResponse.html#method.respond_with_ephemeral_message) | Ephemeral text |
| [`respond_with_embed(embed)`](https://docs.rs/composure/latest/composure/types/enum.InteractionResponse.html#method.respond_with_embed) | One embed |
| [`respond_with_components(components)`](https://docs.rs/composure/latest/composure/types/enum.InteractionResponse.html#method.respond_with_components) | A Components V2 message |
| [`InteractionResponse::ChannelMessageWithSource(data)`](https://docs.rs/composure/latest/composure/types/enum.InteractionResponse.html#variant.ChannelMessageWithSource) | Any [`MessageCallbackData`](https://docs.rs/composure/latest/composure/types/struct.MessageCallbackData.html): content, up to 10 embeds, components, a poll, allowed mentions, flags |
| [`InteractionResponse::Modal(data)`](https://docs.rs/composure/latest/composure/types/enum.InteractionResponse.html#variant.Modal) | A modal ([`ModalCallbackData`](https://docs.rs/composure/latest/composure/types/struct.ModalCallbackData.html)) |

```rust
use composure::{
    command,
    types::{Embed, InteractionResponse, MessageCallbackData, MessageFlags},
};

/// Two embeds, without a notification
#[command]
fn compare() -> InteractionResponse {
    InteractionResponse::ChannelMessageWithSource(MessageCallbackData {
        content: Some("Before and after:".into()),
        embeds: Some(vec![Embed::new().with_title("Before"), Embed::new().with_title("After")]),
        flags: Some(MessageFlags::SuppressNotifications),
        ..Default::default()
    })
}
# fn main() {}
```

A Components V2 message (one with
[`MessageFlags::IsComponentsV2`](https://docs.rs/composure/latest/composure/types/struct.MessageFlags.html#associatedconstant.IsComponentsV2), as built by
`respond_with_components`, [`MessageCallbackData::components_v2`](https://docs.rs/composure/latest/composure/types/struct.MessageCallbackData.html#method.components_v2),
a `Container` or a View) is laid out entirely by its components: Discord rejects `content`,
`embeds` and polls alongside it, so text goes in text displays.

## Discord's images

Avatars, guild icons and banners, role icons, emojis and stickers are served by Discord's
[CDN](https://discord.com/developers/docs/reference#image-formatting). Their types build the
URLs: [`User::display_avatar_url`](https://docs.rs/composure/latest/composure/types/struct.User.html#method.display_avatar_url),
[`Member::guild_avatar_url`](https://docs.rs/composure/latest/composure/types/struct.Member.html#method.guild_avatar_url),
[`Guild::icon_url`](https://docs.rs/composure/latest/composure/types/struct.Guild.html#method.icon_url),
[`Emoji::url`](https://docs.rs/composure/latest/composure/types/struct.Emoji.html#method.url), ...
Each `*_url` has a `*_url_with(format, size)` variant; animated images are GIFs unless another
[`ImageFormat`](https://docs.rs/composure/latest/composure/types/enum.ImageFormat.html) is asked for.

```rust
use composure::{
    command,
    types::{Embed, EmbedThumbnail, User},
};

/// Show someone's avatar
#[command]
fn avatar(#[option("Whose")] user: &User) -> Embed {
    let url = user.display_avatar_url_with(None, Some(256));
    Embed::new()
        .with_title(user.display_name())
        .with_thumbnail(EmbedThumbnail::new(url, None, None, None))
}
# fn main() {}
```

## Files

[`with_file`](https://docs.rs/composure/latest/composure/types/enum.InteractionResponse.html#method.with_file) attaches a
[`File`](https://docs.rs/composure/latest/composure/types/struct.File.html) to a message response. Reference it from an embed or a component
as `attachment://<name>`.

```rust
use composure::{command, types::{File, InteractionResponse}};

/// Get your text back as a file
#[command]
fn export(#[option("What to put in the file", max_len = 2000)] text: &str) -> InteractionResponse {
    InteractionResponse::respond_with_message("Here's your file:".into())
        .with_file(File::new("export.txt", text.as_bytes()).content_type("text/plain"))
}
# fn main() {}
```

A response with files is multipart, which an HTTP reply to Discord cannot be: the Adapter sends
it to the callback endpoint itself and answers Discord's request with `202`. Deferred results
with files are sent with the edit of the original response.

## Errors

A handler that returns `Err` is not answered with Discord's "This interaction failed": the
Adapter logs the error and answers with an ephemeral message instead. What the user reads
depends on the [`HandlerError`](https://docs.rs/composure/latest/composure/command/struct.HandlerError.html) kind:

| Error | The user sees | Logged at |
|---|---|---|
| [`HandlerError::user(msg)`](https://docs.rs/composure/latest/composure/command/struct.HandlerError.html#method.user) | `msg` | DEBUG |
| [`HandlerError::new(msg)`](https://docs.rs/composure/latest/composure/command/struct.HandlerError.html#method.new), any other error | "Something went wrong handling this interaction." | ERROR |
| An Option that could not be read ([`ErrorKind::InvalidOption`](https://docs.rs/composure/latest/composure/command/enum.ErrorKind.html#variant.InvalidOption)) | the same generic message | WARN |
| A Command the App does not list ([`ErrorKind::UnknownCommand`](https://docs.rs/composure/latest/composure/command/enum.ErrorKind.html#variant.UnknownCommand)) | "Unknown command." | WARN |

With a [Theme](theming.md), the message is an embed in the Theme's `error` color.

Any `E: Display` works as the error type, and any [`std::error::Error`](https://doc.rust-lang.org/std/error/trait.Error.html) converts into a
`HandlerError` with `?`, so REST errors, parse errors and the like propagate as they are.
`HandlerError::user` is for errors the user can act on:

```rust
use composure::{command, command::{Ephemeral, HandlerError}};

/// Roll dice, like 2d6
#[command]
fn roll(#[option("Dice, like 2d6")] dice: &str) -> Result<Ephemeral<String>, HandlerError> {
    let (count, sides) = dice
        .split_once('d')
        .ok_or_else(|| HandlerError::user("Write dice like `2d6`."))?;
    let count: u32 = count.parse()?; // a ParseIntError: logged, generic message
    let sides: u32 = sides.parse()?;
    Ok(Ephemeral(format!("Max roll: {}", count * sides)))
}
# fn main() {}
```

A converted error keeps its causes (its [`source`](https://doc.rust-lang.org/std/error/trait.Error.html#method.source)
chain) for the logs: `{}` prints only the message, as before, while `{:#}` and the `sources` log
field show the rest, like `request failed: connection reset`.
[`with_source`](https://docs.rs/composure/latest/composure/command/struct.HandlerError.html#method.with_source)
adds a cause to an error you build yourself, without showing it to the user:

```rust
use composure::{command, command::HandlerError};

/// Read the settings file
#[command]
fn settings() -> Result<String, HandlerError> {
    std::fs::read_to_string("settings.toml")
        .map_err(|error| HandlerError::user("Settings are unavailable.").with_source(error))
}
# fn main() {}
```

An unknown command or an Option that cannot be read usually means Discord's stored Commands
are out of date: [Sync](getting_started.md#syncing-commands) again.

## Your own types

Implement [`IntoResponse`](https://docs.rs/composure/latest/composure/command/trait.IntoResponse.html) to return your own types directly:

```rust
use composure::{command, command::IntoResponse, types::{Embed, InteractionResponse}};

pub struct Forecast { pub city: String, pub celsius: f32 }

impl IntoResponse for Forecast {
    fn into_response(self) -> InteractionResponse {
        Embed::new()
            .with_title(&self.city)
            .with_description(&format!("{:.1} °C", self.celsius))
            .into_response()
    }
}

/// Today's weather
#[command]
fn weather() -> Forecast {
    Forecast { city: "Oslo".into(), celsius: -3.5 }
}
# fn main() {}
```

Override [`try_into_response`](https://docs.rs/composure/latest/composure/command/trait.IntoResponse.html#method.try_into_response) as well when
building the response can fail; handlers call that one.
