Buttons, select menus and modals send their own Interactions when used. Each carries the
`custom_id` the bot gave the component, and Composure routes it to the handler whose **prefix**
starts it: `vote:12:1` goes to the handler for `vote`, which receives `12` and `1` as its state.

This chapter builds components by hand and answers them with
[`#[component]`](crate::component) and [`#[modal]`](crate::modal). [Views](crate::guide::views)
do the same with markup and inline handlers; the two mix freely.

# Component handlers

```
use composure::{
    app, command, component,
    command::{Update, custom_id::CustomIdError},
    types::{ButtonStyle, Component, InteractionResponse, MessageCallbackData},
};

/// A message with a button that counts its clicks
#[command]
fn counter() -> Result<InteractionResponse, CustomIdError> {
    Ok(InteractionResponse::ChannelMessageWithSource(counter_message(0)?))
}

/// The count lives in the button's custom_id, so nothing is stored
#[component("counter")]
fn counter_click(count: u32) -> Result<Update<InteractionResponse>, CustomIdError> {
    Ok(Update(InteractionResponse::ChannelMessageWithSource(counter_message(count + 1)?)))
}

fn counter_message(count: u32) -> Result<MessageCallbackData, CustomIdError> {
    let button = Component::new_button(
        ButtonStyle::Primary,
        Some("+1".into()),
        None,
        Some(counter_click::custom_id(count)?), // "counter:<count>"
        None,
        None,
    );
    Ok(MessageCallbackData {
        content: Some(format!("Clicked {count} times")),
        components: Some(vec![Component::row(vec![button])]),
        ..Default::default()
    })
}

#[app(handlers(counter, counter_click))]
pub struct Bot;
# fn main() {}
```

`#[component("prefix")]` on a function:

- takes an optional leading `ctx: ComponentCtx` (with a State, like a Command's
  [`Ctx`](crate::guide::commands#the-context)), then the **state** parameters
- adds `counter_click::custom_id(state..)`, which encodes the prefix and the state into a
  `custom_id`, or fails with a [`CustomIdError`](crate::command::custom_id::CustomIdError) past
  Discord's 100 characters
- answers like a Command: a new message by default, [`Update(..)`](crate::command::Update) to
  edit the message the component is on, `ctx.defer_update(..)` to edit it later, a modal, ...

The prefix is 1-100 characters without `:` or `\`, and unique within the App (checked at compile
time). Keep it short: it counts toward the 100.

[`ComponentCtx`](crate::command::ComponentCtx) has everything [`Ctx`](crate::command::Ctx) has,
plus [`values()`](crate::command::Ctx::values) for the selected values of a select menu (option
values, or user, role and channel ids). The message the component is on is
[`ctx.interaction().common.message()`](crate::types::InteractionCommon::message), parsed on first
use.

```
use composure::{component, command::{ComponentCtx, Ephemeral}};

/// The selected values of a string select with custom_id "colors"
#[component("colors")]
fn pick_colors(ctx: ComponentCtx) -> Ephemeral<String> {
    Ephemeral(format!("You picked {}", ctx.values().join(", ")))
}
# fn main() {}
```

# State in the `custom_id`

Handlers are stateless: what a handler needs to know about the component that was used is
encoded in its `custom_id` ([ADR 0007](https://github.com/BlueFrog130/composure-rs/blob/main/docs/adr/0007-custom-id-encoding.md)).
The format is `prefix:field:field..`, each field `\`-escaped, written in declaration order
(structs and tuples flatten):

| Type | Encoded as |
|---|---|
| integers, floats, `char`, strings, [`Snowflake`](crate::types::Snowflake) | the value |
| `bool` | `1` / `0` |
| `Option<T>` | empty for `None` (so `Some(String::new())` reads back as `None`) |
| enums | the variant index, then its fields |
| sequences, maps | the length, then the items |

State parameters must be owned (`String`, not `&str`) and implement serde's `Serialize` and
`DeserializeOwned`; for your own structs and enums, derive them. The format does not describe
itself, so changing a handler's state types breaks the components already sent: give the new
version a new prefix if old messages must keep working.

The encoding is available directly as [`custom_id::encode`](crate::command::custom_id::encode),
[`decode`](crate::command::custom_id::decode) and
[`split`](crate::command::custom_id::split).

# Modals

A modal is a form Discord shows over the chat. It is opened as the *response* to a Command or
component (not sent as a message), and its submit is a new Interaction, answered by a
[`#[modal]`](crate::modal) handler.

## Forms

[`#[derive(Modal)]`](crate::Modal) describes a form as a struct: each field is one input inside a
labeled [`Label`](crate::types::Label), and its type decides whether it is required and how
its value is read.

```
use composure::{Choice, Modal, app, command, modal,
    command::{Ephemeral, custom_id::CustomIdError}, types::{Attachment, InteractionResponse}};

#[derive(Choice, Debug)]
pub enum Kind { Bug, Idea, #[choice(name = "Something nice")] Praise }

#[derive(Modal)]
#[modal(title = "Feedback")]
pub struct FeedbackForm {
    #[input(max_len = 100, placeholder = "What is it about?")]
    topic: String,
    #[radio(label = "Kind")]
    kind: Kind,
    #[input(paragraph, max_len = 1000, description = "Optional")]
    details: Option<String>,
    #[file_upload(max = 3, file_types("image"))]
    screenshots: Vec<Attachment>,
    #[checkbox(label = "You may contact me about this")]
    contact: bool,
}

/// Send feedback
#[command]
fn feedback() -> Result<InteractionResponse, CustomIdError> {
    on_feedback::show() // the modal response opening the form
}

/// The submitted form arrives typed
#[modal("feedback")]
fn on_feedback(form: FeedbackForm) -> Ephemeral<String> {
    Ephemeral(format!(
        "{:?} **{}**: {} ({} screenshots, contact: {})",
        form.kind, form.topic, form.details.as_deref().unwrap_or("-"),
        form.screenshots.len(), form.contact,
    ))
}

#[app(handlers(feedback, on_feedback))]
pub struct Bot;
# fn main() {}
```

On the struct, `#[modal(title = "...", text = "markdown")]`: the title (max 45 characters) and
text displays shown above the inputs (`text` may repeat). At most 5 fields and texts together.

Each field takes one attribute naming its input (`#[input]` when there is none):

| Attribute | Input | Settings besides `label`, `description`, `id` |
|---|---|---|
| `#[input]` | [`TextInput`](crate::types::TextInput) | `paragraph`, `min_len`, `max_len`, `placeholder`, `value` |
| `#[select]` | [`StringSelect`](crate::types::StringSelect) | `placeholder`, `min`, `max`, `option("Label", "value")`.. |
| `#[user_select]`, `#[role_select]`, `#[mentionable_select]` | entity selects | `placeholder`, `min`, `max` |
| `#[channel_select]` | [`ChannelSelect`](crate::types::ChannelSelect) | those, and `channel_types(GuildText, ..)` |
| `#[radio]` | [`RadioGroup`](crate::types::RadioGroup) | `option("Label", "value")`.. |
| `#[checkboxes]` | [`CheckboxGroup`](crate::types::CheckboxGroup) | `min`, `max`, `option("Label", "value")`.. |
| `#[checkbox]` | [`Checkbox`](crate::types::Checkbox) | `default` |
| `#[file_upload]` | [`FileUpload`](crate::types::FileUpload) | `min`, `max`, `file_types("image", ".pdf")` |

- `label` is the title above the input (default: the field name), `description` a line under
  it, `id` the input's `custom_id` (default: the field name).
- Without `option(..)`s, a [`#[derive(Choice)]`](crate::guide::commands#choices) field type
  supplies the options.
- The field type ([`ModalValue`](crate::command::ModalValue)) is `String`, `bool`,
  [`Snowflake`](crate::types::Snowflake), [`User`](crate::types::User),
  [`Role`](crate::types::Role), [`PartialChannel`](crate::types::PartialChannel),
  [`Attachment`](crate::types::Attachment) or a Choice enum, alone (required), in an `Option`
  (optional) or in a `Vec` (optional, several). A type the input cannot submit (a `bool` text
  input, a `Vec` radio group) is a compile error.

## Modal handlers

`#[modal("prefix")]` on a function taking an optional `ctx: ModalCtx`, then the form, then
state parameters (as for components). It adds:

- `name::show(state..)`: the [`InteractionResponse`](crate::types::InteractionResponse) opening
  the form, with the state in its `custom_id`
- `name::custom_id(state..)`: the `custom_id` alone, for a modal built some other way

[`ModalCtx`](crate::command::ModalCtx) reads submitted values by `custom_id` too:
[`value(id)`](crate::command::Ctx::value) for text inputs and radio groups,
[`values(id)`](crate::command::Ctx::values) for selects, checkbox groups and uploads,
[`checked(id)`](crate::command::Ctx::checked) for checkboxes.

A modal opened from a component can [`Update`](crate::command::Update) that component's message
when submitted.

# Fallback

Components whose prefix no listed handler claims (left over from an older version, or handled
some other way) get a "not handled" ephemeral message. `Bot.with_fallback(handler)` sends them,
and modals, autocomplete and Commands nothing claims, to any
[`InteractionHandler`](crate::command::InteractionHandler) instead; see
[`WithFallback`](crate::command::WithFallback).

```
use composure::{app, command, command::{HandlerError, InteractionHandler},
    types::{InteractionResponse, MessageComponentInteraction}};

/// Old buttons
struct Legacy;

impl<S: Sync> InteractionHandler<S> for Legacy {
    async fn component(
        &self,
        component: MessageComponentInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        Ok(InteractionResponse::respond_with_ephemeral_message(format!(
            "`{}` is from an older version; run the command again.",
            component.data.custom_id
        )))
    }
}

/// Ping
#[command]
fn ping() -> &'static str { "Pong" }

#[app(handlers(ping))]
pub struct Bot;

# fn main() {
let handler = Bot.with_fallback(Legacy); // hand this to the Adapter
# let _ = handler;
# }
```
