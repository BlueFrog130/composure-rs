A [`Theme`](crate::command::Theme) gives everything a bot says the same colors and branding:
set it once on an App, and embeds, Components V2 containers and error messages are filled in
with it wherever a handler left them unset.

```
use composure::{app, command, command::{Ctx, Theme}, types::Embed};

/// The bot's colors and branding
pub const THEME: Theme = Theme::new()
    .with_primary(0xf5c2e7)
    .with_success(0xa6e3a1)
    .with_error(0xf38ba8)
    .with_author("Composure", Some("https://example.com/icon.png"))
    .with_footer("composure-rs", None);

/// Show the latest release
#[command]
fn release() -> Embed {
    // No color, author or footer set: the Theme fills them in
    Embed::new().with_title("v0.1.0").with_description("Views, Themes, Webhook Events")
}

/// Save your settings
#[command]
fn save(ctx: Ctx) -> Embed {
    // The Theme's helpers build embeds in its semantic colors
    ctx.theme().success("Saved.")
}

#[app(handlers(release, save), theme = THEME)]
pub struct Bot;
# fn main() {}
```

# Defining a Theme

A `Theme` is a `const`: start from [`Theme::new()`](crate::command::Theme::new) (Discord's own
palette, no branding) and override what you need.

| Field | Default | Builder | Used for |
|---|---|---|---|
| `primary` | `0x5865F2` (blurple) | [`with_primary`](crate::command::Theme::with_primary) | Embeds without a color, container accents, [`embed()`](crate::command::Theme::embed) |
| `success` | `0x57F287` (green) | [`with_success`](crate::command::Theme::with_success) | [`success(..)`](crate::command::Theme::success) |
| `warning` | `0xFEE75C` (yellow) | [`with_warning`](crate::command::Theme::with_warning) | [`warning(..)`](crate::command::Theme::warning) |
| `error` | `0xED4245` (red) | [`with_error`](crate::command::Theme::with_error) | [`error(..)`](crate::command::Theme::error), error messages |
| `info` | `0x5865F2` (blurple) | [`with_info`](crate::command::Theme::with_info) | [`info(..)`](crate::command::Theme::info) |
| `author` | none | [`with_author(name, icon_url)`](crate::command::Theme::with_author) | The first embed's author |
| `footer` | none | [`with_footer(text, icon_url)`](crate::command::Theme::with_footer) | The last embed's footer |

Colors are `0xRRGGBB`. [`Theme::DEFAULT`](crate::command::Theme::DEFAULT) is the unmodified
palette.

# Setting it on an App

`#[app(.., theme = THEME)]` takes a path to a `Theme` constant (it is borrowed as
`&'static Theme`). From then on, for every Interaction the App answers:

- **Message responses** (new messages and [`Update`](crate::command::Update)s of the message a
  component is on) are filled in where unset:
  - every embed without a color gets `primary`
  - the first embed gets the `author`, if the Theme has one and the embed has none
  - the last embed gets the `footer`, likewise
  - every top-level [`Container`](crate::types::Container) without an `accent_color` gets
    `primary`
- **Deferred results**: what [`ctx.defer(..)`](crate::command::Ctx::defer) and
  [`defer_update(..)`](crate::command::Ctx::defer_update) work returns is filled in the same
  way before it edits the original response.
- **Errors** become an ephemeral embed in the `error` color, branded, instead of plain text
  (see [Responses](crate::guide::responses#errors)).

Values a handler set itself always win: an embed with a color keeps it, a container with an
`accent_color` keeps it.

Nothing else is touched: plain text messages (no embed to fill in), modals, autocomplete
suggestions, and anything sent through [REST](crate::guide::deferring) (follow-ups, channel
messages) go out exactly as built. Build those with the helpers below to keep them on-theme.

# Using it in handlers

[`ctx.theme()`](crate::command::Ctx::theme) returns the Theme of the App answering (the
[`DEFAULT`](crate::command::Theme::DEFAULT) palette when there is none), in Commands, component,
modal and autocomplete handlers, inline View handlers, and Background Work
([`DeferredCtx::theme`](crate::command::DeferredCtx::theme)).

| Helper | Builds |
|---|---|
| [`embed()`](crate::command::Theme::embed) | An empty embed in `primary`, with the author and footer |
| [`success(message)`](crate::command::Theme::success) | A branded embed with `message` as its description, in `success` |
| [`warning(message)`](crate::command::Theme::warning) | The same, in `warning` |
| [`error(message)`](crate::command::Theme::error) | The same, in `error` |
| [`info(message)`](crate::command::Theme::info) | The same, in `info` |

```
use composure::{command, command::{Ctx, Ephemeral, HandlerError}, types::User};

/// Ban a member
#[command]
fn ban(ctx: Ctx, #[option("Who")] user: &User) -> Result<Ephemeral<composure::types::Embed>, HandlerError> {
    if ctx.user().is_some_and(|me| me.id == user.id) {
        return Err(HandlerError::user("You can't ban yourself."));
    }
    Ok(Ephemeral(ctx.theme().warning(format!("Would ban {}", user.display_name()))))
}
# fn main() {}
```

# Themes and components

Components V2 messages ([Views](crate::guide::views), or
[`InteractionResponse::respond_with_components`](crate::types::InteractionResponse::respond_with_components))
cannot carry embeds, so a Theme reaches them through their containers: each top-level
`<Container>` left without an `accent_color` gets `primary`. Containers nested elsewhere and
other components have no color to fill.

For the semantic colors, set the accent from the Theme yourself:

```
use composure::{command, rsx, command::{Ctx, View}};

/// Check the deploy
#[command]
fn deploy(ctx: Ctx) -> View {
    let theme = ctx.theme();
    let healthy = true;
    rsx! {
        <Container>
            // primary, filled in by the App's Theme
            "## Deploy"
        </Container>
        <Container accent_color={if healthy { theme.success } else { theme.error }}>
            {if healthy { "All checks passed" } else { "Checks failed" }}
        </Container>
    }
}
# fn main() {}
```

Outside handlers (a `#[view]` building its markup from Props alone), name the `Theme` constant
directly: `accent_color=THEME.warning`.

# Nested Apps

Apps nest (see [Getting started](crate::guide::getting_started#apps)), and each can set its own
Theme:

- A Route's responses are filled in by the closest App listing it that has a Theme, then by
  each App above it for whatever is still unset (an inner Theme without a footer gets the outer
  one's footer).
- `ctx.theme()` is the closest App's Theme.
- An App without a Theme passes the outer one through.
- Errors are rendered with the outermost App's Theme, since the Adapter renders them.

```
use composure::{app, command, command::Theme};

const ADMIN: Theme = Theme::new().with_primary(0xED4245).with_author("Admin", None);
const BOT: Theme = Theme::new().with_footer("my-bot", None);

/// Wipe the cache
#[command]
fn wipe() -> composure::types::Embed {
    composure::types::Embed::new().with_title("Wiped") // red, "Admin" author, "my-bot" footer
}

#[app(handlers(wipe), theme = ADMIN)]
pub struct Admin;

#[app(handlers(Admin), theme = BOT)]
pub struct Bot;
# fn main() {}
```

# Opting out

Wrap a response in [`Unthemed`](crate::command::Unthemed) to send it exactly as built. It
composes with the other wrappers: `Ephemeral(Unthemed(embed))`, `Update(Unthemed(view))`.

```
use composure::{command, command::Unthemed, types::Embed};

/// Show the raw embed
#[command]
fn raw() -> Unthemed<Embed> {
    Unthemed(Embed::new().with_title("No color, no branding"))
}
# fn main() {}
```

A handler implementing [`InteractionHandler`](crate::command::InteractionHandler) by hand
themes its responses through
[`InteractionHandler::theme`](crate::command::InteractionHandler::theme), which
[`dispatch`](crate::command::dispatch) applies, and [`Theme::apply`](crate::command::Theme::apply)
does the filling for anything else.
