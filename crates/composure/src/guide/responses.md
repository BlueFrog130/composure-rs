What a handler returns is how it answers. Commands, component and modal handlers, and inline
View handlers all answer the same way: return anything implementing
[`IntoResponse`](crate::command::IntoResponse), or a `Result` of one.

```
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
[REST and deferring](crate::guide::deferring).

# What can be returned

| Type | Answer |
|---|---|
| `String`, `&str` | A message with that content |
| [`Embed`](crate::types::Embed) | A message with one embed |
| [`Container`](crate::types::Container) | A Components V2 message of one container |
| [`View`](crate::command::View), a `#[view]`'s Props | A Components V2 message ([Views](crate::guide::views)) |
| [`ModalView`](crate::command::ModalView), a modal `#[view]`'s Props | Opens a modal |
| [`InteractionResponse`](crate::types::InteractionResponse) | Exactly that; anything Discord accepts |
| [`Deferred`](crate::command::Deferred) | "Thinking...", then the result of Background Work ([`ctx.defer`](crate::command::Ctx::defer)) |
| [`DeferredUpdate`](crate::command::DeferredUpdate) | Nothing visible yet, then an edit of the component's message ([`ctx.defer_update`](crate::command::Ctx::defer_update)) |
| [`Ephemeral<T>`](crate::command::Ephemeral) | `T`, visible only to the user who invoked it |
| [`Update<T>`](crate::command::Update) | `T` as an edit of the message the component is on (components and modals opened from one) |
| [`Unthemed<T>`](crate::command::Unthemed) | `T` without the App's [Theme](crate::guide::theming) filled in |
| `Result<T, E>` with `E: Display` | `T`, or an [error message](#errors) |

The wrappers compose: `Ephemeral(Unthemed(embed))`, `Update(view)`,
`Ephemeral(ctx.defer(..))`, `Result<Ephemeral<String>, HandlerError>`.

`Ephemeral` on a deferral makes the "thinking" message ephemeral, and the final message with
it: ephemerality is decided when the Interaction is first answered and cannot change later.

# Building messages by hand

[`InteractionResponse`](crate::types::InteractionResponse) is Discord's
[interaction response](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-response-object),
with constructors for the common shapes:

| Constructor | |
|---|---|
| [`respond_with_message(content)`](crate::types::InteractionResponse::respond_with_message) | Text |
| [`respond_with_ephemeral_message(content)`](crate::types::InteractionResponse::respond_with_ephemeral_message) | Ephemeral text |
| [`respond_with_embed(embed)`](crate::types::InteractionResponse::respond_with_embed) | One embed |
| [`respond_with_components(components)`](crate::types::InteractionResponse::respond_with_components) | A Components V2 message |
| [`InteractionResponse::ChannelMessageWithSource(data)`](crate::types::InteractionResponse::ChannelMessageWithSource) | Any [`MessageCallbackData`](crate::types::MessageCallbackData): content, up to 10 embeds, components, a poll, allowed mentions, flags |
| [`InteractionResponse::Modal(data)`](crate::types::InteractionResponse::Modal) | A modal ([`ModalCallbackData`](crate::types::ModalCallbackData)) |

```
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
[`MessageFlags::IsComponentsV2`](crate::types::MessageFlags::IsComponentsV2), as built by
`respond_with_components`, [`MessageCallbackData::components_v2`](crate::types::MessageCallbackData::components_v2),
a `Container` or a View) is laid out entirely by its components: Discord rejects `content`,
`embeds` and polls alongside it, so text goes in text displays.

# Files

[`with_file`](crate::types::InteractionResponse::with_file) attaches a
[`File`](crate::types::File) to a message response. Reference it from an embed or a component
as `attachment://<name>`.

```
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

# Errors

A handler that returns `Err` is not answered with Discord's "This interaction failed": the
Adapter logs the error and answers with an ephemeral message instead. What the user reads
depends on the [`HandlerError`](crate::command::HandlerError) kind:

| Error | The user sees | Logged at |
|---|---|---|
| [`HandlerError::user(msg)`](crate::command::HandlerError::user) | `msg` | DEBUG |
| [`HandlerError::new(msg)`](crate::command::HandlerError::new), any other error | "Something went wrong handling this interaction." | ERROR |
| An Option that could not be read ([`ErrorKind::InvalidOption`](crate::command::ErrorKind::InvalidOption)) | the same generic message | WARN |
| A Command the App does not list ([`ErrorKind::UnknownCommand`](crate::command::ErrorKind::UnknownCommand)) | "Unknown command." | WARN |

With a [Theme](crate::guide::theming), the message is an embed in the Theme's `error` color.

Any `E: Display` works as the error type, and any [`std::error::Error`] converts into a
`HandlerError` with `?`, so REST errors, parse errors and the like propagate as they are.
`HandlerError::user` is for errors the user can act on:

```
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

An unknown command or an Option that cannot be read usually means Discord's stored Commands
are out of date: [Sync](crate::guide::getting_started#syncing-commands) again.

# Your own types

Implement [`IntoResponse`](crate::command::IntoResponse) to return your own types directly:

```
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

Override [`try_into_response`](crate::command::IntoResponse::try_into_response) as well when
building the response can fail; handlers call that one.
