# Views

A View is a message or modal layout written as markup with [`rsx!`](https://docs.rs/composure/latest/composure/macro.rsx.html), and
optionally a function of Props with [`#[view]`](https://docs.rs/composure/latest/composure/attr.view.html) whose buttons, selects and modal
submits are handled inline. Markup builds the same [`types`](https://docs.rs/composure/latest/composure/types/) a hand-written
response would, so everything here can also be done with
[`Component`](https://docs.rs/composure/latest/composure/types/enum.Component.html) constructors; markup is shorter, checks placement at
compile time, and keeps a component next to the code that answers it.

```rust
use composure::{app, command, rsx, view, command::View};

/// A card that counts its likes; the count is carried in the button's `custom_id`
#[view]
pub fn Card(title: String, likes: u32) -> View {
    rsx! {
        <Container>
            "## " {title.as_str()}
            <Text>"-# " {likes} " likes"</Text>
            <Row>
                <Button primary on:click:like=move |_| Card { likes: likes + 1, title }>
                    "Like"
                </Button>
            </Row>
        </Container>
    }
}

/// Show a card
#[command]
fn card() -> Card {
    Card { title: "Hello".into(), likes: 0 }
}

#[app(handlers(card, Card))]
pub struct Bot;
# fn main() {}
```

`/card` answers with the card. Clicking **Like** runs the closure, which returns new Props; the
message the button is on is re-rendered with them. Nothing is stored anywhere: `title` and
`likes` travel in the button's `custom_id`.

- [Markup](#markup): `rsx!`, text, `{expressions}`, attributes
- [`#[view]`](#view-functions): Props, sub-Views, children
- [Inline handlers](#inline-handlers): `on:click`, `on:select`, `on:submit`
- [Modal Views](#modal-views): forms read back as typed parameters
- [Message options](#message-options): ephemeral, allowed mentions, files
- [Tag reference](#tag-reference): every tag and its attributes
- [Styling](#styling): colors, tone Views, and how errors look
- [Without `#[view]`](#without-view): markup in plain functions, sending Views through REST

## Markup

`rsx!` takes JSX-like markup and evaluates to a [`View`](https://docs.rs/composure/latest/composure/command/struct.View.html) (a Components V2
message) or, when its only root is `<Modal>`, a [`ModalView`](https://docs.rs/composure/latest/composure/command/struct.ModalView.html). Both
implement [`IntoResponse`](https://docs.rs/composure/latest/composure/command/trait.IntoResponse.html), so a Command can return them
directly.

```rust
use composure::{command, rsx, command::View};

/// Show a status panel
#[command]
fn status() -> View {
    let healthy = 3;
    let total = 4;
    rsx! {
        <Container accent_color=0x57F287>
            "## Status"
            <Separator/>
            "Healthy: **" {healthy} "/" {total} "**"
        </Container>
    }
}
# fn main() {}
```

### Tags

PascalCase tags are either built-in (Discord's components, listed in the
[tag reference](#tag-reference)) or the name of a `#[view]` function placed as a
[sub-View](#sub-views). Unknown lowercase tags are a compile error. Tags are self-closing
(`<Separator/>`) or have children (`<Row>..</Row>`).

Placement follows Discord's rules and is checked at compile time:

| Where | What can go there |
|---|---|
| Top of a message | text, `<Text>`, `<Container>`, `<Section>`, `<Gallery>`, `<File>`, `<Separator>`, `<Row>` |
| `<Container>` | the same, except another `<Container>` |
| `<Section>` | text and `<Text>` (1-3), plus one accessory: a `<Button>` or `<Thumbnail>` |
| `<Row>` | up to 5 `<Button>`s, or one select |
| `<Gallery>` | `<Item>`s |
| select, `<RadioGroup>`, `<CheckboxGroup>` | `<Option>`s |
| `<Modal>` (the only root) | text, `<Text>`, `<Label>` |
| `<Label>` | exactly one input: `<TextInput>`, a select, `<RadioGroup>`, `<CheckboxGroup>`, `<Checkbox>`, `<FileUpload>` |

A sub-View's markup may also be a single `<Button>` or select meant for a row.

Discord's limits are checked, so breaking one is an error that names it instead of Discord's
bare "This interaction failed". What the markup shows is checked at compile time: more than 5
buttons in a `<Row>` (or a select with anything beside it), more than 3 texts in a `<Section>`,
more than 5 children in a `<Modal>`, more than 25 `<Option>`s in a select (10 in a group) or 10
`<Item>`s in a `<Gallery>`. The rest (40 components in a message, 4000 characters of text,
`custom_id`s past 100 characters, button labels past 80, levels filled by `{..}`) is checked in
one walk when the View is answered with or sent through REST, and fails as a
[`ViewError`](https://docs.rs/composure/latest/composure/command/enum.ViewError.html) (a `HandlerError` in handlers). The limits are
[`types::limits`](https://docs.rs/composure/latest/composure/types/limits/).

### Text

String literals are text. Adjacent literals and `{expressions}` join into one
[`TextDisplay`](https://docs.rs/composure/latest/composure/types/struct.TextDisplay.html), so these are the same:

```rust
# use composure::rsx;
# let name = "Ada";
# let a =
rsx! { "Hello, **" {name} "**!" }
# ;
# let b =
rsx! { <Text>"Hello, **" {name} "**!"</Text> }
# ;
# assert_eq!(format!("{:?}", a.components()), format!("{:?}", b.components()));
```

Text is [Discord markdown](https://support.discord.com/hc/en-us/articles/210298617): `#`
headings, `-#` subtext, `**bold**`, lists, links, mentions. A `<Text>` (or a component) ends the
current run of text; use separate `<Text>` tags for separate text displays, for instance the 1-3
of a `<Section>`.

Text cannot go in a `<Row>`. Bare words without quotes are a compile error
(`put text in quotes`).

### Expressions

`{expr}` evaluates Rust inside markup. Where components can go, the value is anything
implementing [`IntoChildren`](https://docs.rs/composure/latest/composure/command/trait.IntoChildren.html):

| Value | Becomes |
|---|---|
| `&str`, `String`, `&String`, `char`, integers, floats, [`Snowflake`](https://docs.rs/composure/latest/composure/types/struct.Snowflake.html), `format_args!(..)` | text, joined with adjacent text |
| [`Component`](https://docs.rs/composure/latest/composure/types/enum.Component.html) and every component struct ([`Container`](https://docs.rs/composure/latest/composure/types/struct.Container.html), [`ButtonComponent`](https://docs.rs/composure/latest/composure/types/struct.ButtonComponent.html), ...) | that component |
| [`View`](https://docs.rs/composure/latest/composure/command/struct.View.html), a `#[view]`'s Props | its components, in place |
| `Option<T>` | nothing for `None` |
| `Vec<T>`, arrays, iterator adapters (`map`, `filter`, `filter_map`, `flat_map`, `flatten`, `chain`, `take`, `skip`, `rev`, `once`, `vec::IntoIter`) | each item in order |

So lists are a `map`, and conditionals are an `Option`:

```rust
use composure::{rsx, command::View};

fn shopping(items: &[&str], note: Option<&str>) -> View {
    rsx! {
        <Container>
            "## Shopping list"
            {items.iter().map(|item| rsx! { <Text>"- " {*item}</Text> })}
            {note.map(|note| rsx! { <Separator/> <Text>"-# " {note}</Text> })}
        </Container>
    }
}
# fn main() { assert_eq!(shopping(&["eggs"], None).components().len(), 1); }
```

Other iterators (`Peekable`, `Zip`, ...) need `.collect::<Vec<_>>()` first.

In text-only places (`<Text>`, a `<Button>`'s label, an `<Option>`'s label) the value is
anything implementing [`IntoText`](https://docs.rs/composure/latest/composure/command/trait.IntoText.html): strings, numbers, `Snowflake`, and
`Option`s of them. Call `.to_string()` on other `Display` types.

### Attributes

Attributes are written `name=value`; the value is a Rust expression (a literal, a variable, a
call, a block `{..}`). A bare attribute is `true`: `<Button disabled>` is
`<Button disabled=true>`. Each attribute may appear once.

Apart from the few attributes a tag handles itself (marked *special* in the
[reference](#tag-reference): `url`, `on:click`, the style flags, ...), every attribute sets
the field of the same name on the component's struct. That means:

- every public field of the component's type is an attribute, including `id`
- the value is converted with [`IntoAttr`](https://docs.rs/composure/latest/composure/command/trait.IntoAttr.html): a `&str` works for a
  `String` or `Option<String>` field, a `T` for an `Option<T>` field, `"👍"` for an emoji
- an attribute the type does not have is a compile error naming the type
  (`no field `colour` on type `Container``)

Values that contain `<..>` (generics, closures with typed parameters) must be wrapped in
braces, or the markup parser reads `<` as a tag: `options={Vec::<SelectOption>::new()}`.

### Comments

`<!-- comments -->` are allowed anywhere and produce nothing.

### Errors

`rsx!` reports every error in the markup at once, not just the first: a misplaced tag, a bad
attribute, an unclosed or mismatched tag. A tag with an error is left out and the rest is still
expanded, so the `{..}` blocks around it keep type-checking, and rust-analyzer keeps completing
them while a tag is half-typed. A `#[view]` with markup errors still gets its Props, builder and
handlers, so code using the View doesn't report errors of its own.

Built-in tags expand to their types at the tag's own span, so hovering `<Container>` (or
`</Container>`) shows the type's docs and go-to-definition opens it. rust-analyzer also completes
tag names and the special attributes (`<Button pri` offers `primary`) and shows their docs on
hover, from a hidden module the macro refers to; field attributes complete and document as the
fields they set.

## View functions

`#[view]` turns a function returning `View` (or `ModalView`) into a component of its own. The
parameters are its Props, which must be owned types:

```rust
use composure::{rsx, view, command::View};

/// A labeled progress bar
#[view]
pub fn Progress(label: String, done: u32, #[prop(default = 10)] total: u32, note: Option<String>) -> View {
    let filled = (done * 10 / total.max(1)).min(10) as usize;
    rsx! {
        <Text>
            "**" {label.as_str()} "** " {"▰".repeat(filled)} {"▱".repeat(10 - filled)}
            {note.map(|note| format!("\n-# {note}"))}
        </Text>
    }
}
# fn main() {}
```

The macro keeps the function body and adds:

- a **Props struct** with the same name and one public field per parameter:
  `Progress { label, done, total, note }`. Returning it from a Command (or a handler) sends the
  View; `Progress { .. }.render()` gives the `View` itself.
- a **builder**, `Progress::builder().label("Build").done(3).build()`, which is what markup
  uses. Setters take anything [`IntoAttr`](https://docs.rs/composure/latest/composure/command/trait.IntoAttr.html) converts, and leaving out
  a required Prop is a compile error.
- [`IntoResponse`](https://docs.rs/composure/latest/composure/command/trait.IntoResponse.html), [`IntoChildren`](https://docs.rs/composure/latest/composure/command/trait.IntoChildren.html),
  [`Rerender`](https://docs.rs/composure/latest/composure/command/trait.Rerender.html) for the Props, and the
  [`ViewHandler`](https://docs.rs/composure/latest/composure/command/trait.ViewHandler.html) running its inline handlers.

Props:

| Prop | In markup |
|---|---|
| `name: T` | required |
| `name: Option<T>` | optional, `None` when left out; `name=value` sets `Some(value)` |
| `#[prop(default = expr)] name: T` | optional, `expr` when left out |
| `children: View` | the tag's children (see [below](#children)); not usable in handlers |

### Sub-Views

A View is placed in markup as a tag, with its Props as attributes:

```rust
# use composure::{rsx, view, command::View};
# #[view]
# pub fn Progress(label: String, done: u32, #[prop(default = 10)] total: u32, note: Option<String>) -> View {
#     rsx! { <Text>{label.as_str()}</Text> }
# }
#[view]
pub fn Build(step: u32) -> View {
    rsx! {
        <Container>
            "## Build"
            <Progress label="Compile" done=step/>
            <Progress label="Test" done={step.saturating_sub(3)} total=7 note="flaky"/>
        </Container>
    }
}
# fn main() {}
```

Tag names may be paths (`<widgets::Progress done=1 label="x"/>`). A sub-View with inline
handlers must be listed in the same `#[app]` as the View placing it (or in any App of the tree);
forgetting it is a compile error from `#[app]`.

### Children

A `children: View` Prop receives the markup between the tags, which lets a View wrap others:

```rust
use composure::{rsx, view, command::View};

/// A titled frame
#[view]
pub fn Panel(title: String, children: View) -> View {
    rsx! {
        <Container>
            "### " {title}
            <Separator/>
            {children}
        </Container>
    }
}

# fn main() {
let view = rsx! {
    <Panel title="Inbox">
        <Text>"No new messages"</Text>
    </Panel>
};
# assert_eq!(view.components().len(), 1);
# }
```

## Inline handlers

An `on:<event>:<name>=closure` attribute gives a component its own handler, inside the View that
renders it. The macro turns each into an arm of the View's handler, routed by the `custom_id`
`<prefix>:<name>:<state>`:

- `<prefix>` is the View's name in `snake_case` (`Card` → `card`); change it with
  `#[view(prefix = "c")]` to save characters
- `<name>` is the handler's name, unique within the View
- `<state>` holds the Props the closure uses, found by reading its body (`format!("{name}")`
  counts), encoded with [`custom_id`](https://docs.rs/composure/latest/composure/command/custom_id/)

When the component is used, the Props are decoded and the closure runs with them in scope. Its
return value answers the Interaction:

- a View's Props or a [`View`](https://docs.rs/composure/latest/composure/command/struct.View.html) (or a `Result` of one) **re-renders** the
  message the component is on (an [`Update`](https://docs.rs/composure/latest/composure/command/struct.Update.html))
- anything else a Command can return ([`CommandOutput`](https://docs.rs/composure/latest/composure/command/trait.CommandOutput.html)): a new
  message, `Ephemeral(..)`, a `ModalView` to open a modal, `ctx.defer_update(..)`, ...

| Event | Tags | Closure |
|---|---|---|
| `on:click:name` | `<Button>` | `move \|ctx\| ..` |
| `on:select:name` | selects | `move \|ctx, values: T\| ..` |
| `on:submit:name` | `<Modal>` | `move \|ctx, field: T, ..\| ..` ([Modal Views](#modal-views)) |

`ctx` is a [`ComponentCtx`](https://docs.rs/composure/latest/composure/command/type.ComponentCtx.html) (a
[`ModalCtx`](https://docs.rs/composure/latest/composure/command/type.ModalCtx.html) for submits); write `_` when unused. Untyped, it works with
any State; annotate it to require one, as Commands do:
`{move |ctx: ComponentCtx<impl Platform>| ..}` (braced, because of the `<..>`). Closures may be
`async`.

```rust
use composure::{rsx, view, command::{ComponentCtx, Ephemeral, Platform, View}};

#[view]
pub fn Vote(question: String, yes: u32, no: u32) -> View {
    rsx! {
        <Text>"**" {question.as_str()} "**\n" {yes} " yes, " {no} " no"</Text>
        <Row>
            <Button success on:click:yes=move |_| Vote { yes: yes + 1, question, no }>"Yes"</Button>
            <Button danger on:click:no=move |_| Vote { no: no + 1, question, yes }>"No"</Button>
            <Button on:click:who=move |ctx| {
                Ephemeral(format!("You are {}", ctx.user().map_or("nobody", |u| u.display_name())))
            }>"Who am I?"</Button>
        </Row>
    }
}
# fn main() {}
```

Things to know:

- The `custom_id` is limited to 100 characters, prefix and handler name included. Past that,
  answering with the View fails with a [`HandlerError`](https://docs.rs/composure/latest/composure/command/struct.HandlerError.html) (the
  error rides inside the View until then). Keep captured Props small: ids and counters, not
  text.
- Captured Props must implement `serde::Serialize` and `DeserializeOwned`; integers, `bool`,
  `String`, `Snowflake`, `Option`s, tuples, and `#[derive(Serialize, Deserialize)]` types do.
- A `children` Prop cannot be captured: it is markup, not data.
- A handler only sees Props. Using a local of the `#[view]` body (`let total = ..`) or a
  closure's parameter around the markup (`{items.iter().map(|item| rsx! { .. })}`) is a compile
  error at its use (`total` is not a Prop), as is a local that shadows a Prop. Make it a Prop, or
  move the markup into a [sub-View](#sub-views) that takes it as one.
- List every View with inline handlers in an `#[app]`; that is how its `custom_id`s are routed.

### Selects

An `on:select` closure takes the selected values as its second parameter, typed with
[`SelectValue`](https://docs.rs/composure/latest/composure/command/trait.SelectValue.html): `String`, `Snowflake`, a resolved
[`User`](https://docs.rs/composure/latest/composure/types/struct.User.html), [`Role`](https://docs.rs/composure/latest/composure/types/struct.Role.html) or
[`PartialChannel`](https://docs.rs/composure/latest/composure/types/struct.PartialChannel.html), or a `#[derive(Choice)]` enum, each alone
(one required selection), in an `Option` (none allowed) or a `Vec` (several; `max` then defaults
to every option).

```rust
use composure::{Choice, rsx, view, command::View};

#[derive(Choice, Debug)]
pub enum Topping { Cheese, Olives, Basil }

#[view]
pub fn Pizza(picked: String) -> View {
    rsx! {
        <Text>"Toppings: " {picked.as_str()}</Text>
        <Row>
            <StringSelect placeholder="Pick toppings" options=Topping
                on:select:pick={move |_ctx, toppings: Vec<Topping>| Pizza {
                    picked: format!("{toppings:?}"),
                }}/>
        </Row>
    }
}
# fn main() {}
```

### Handlers defined elsewhere

`on:click`, `on:select` and `on:submit` without a name point at a
[`#[component]`](https://docs.rs/composure/latest/composure/attr.component.html) or [`#[modal]`](https://docs.rs/composure/latest/composure/attr.modal.html) handler instead, with the
state to put in its `custom_id`: `on:click=vote(poll, true)` sets
`custom_id = vote::custom_id(poll, true)`, and `on:click=refresh` calls
`refresh::custom_id()`. These work in any `rsx!`, not only in `#[view]`s. Or set `custom_id=..`
yourself and answer it however you like.

## Modal Views

A `#[view]` returning `ModalView` with a `<Modal>` root is a form. With an inline `on:submit`,
each input's `name` matches one of the closure's parameters, which receive the submitted values
typed:

```rust
use composure::{Choice, app, command, rsx, view, command::{Ephemeral, ModalView}};

#[derive(Choice, Debug)]
pub enum Severity { Low, High }

#[view]
pub fn Report(page: String) -> ModalView {
    rsx! {
        <Modal title="Report a problem" on:submit:send={move |_ctx, what: String, severity: Severity, notify: bool| {
            Ephemeral(format!("{page}: {what} ({severity:?}, notify: {notify})"))
        }}>
            <Text>"Reporting **" {page.as_str()} "**"</Text>
            <Label text="What happened?">
                <TextInput name="what" paragraph max_len=500/>
            </Label>
            <Label text="Severity">
                <RadioGroup name="severity" options=Severity/>
            </Label>
            <Label text="Notify me">
                <Checkbox name="notify"/>
            </Label>
        </Modal>
    }
}

/// Report a problem with a page
#[command]
fn report(#[option("Which page")] page: String) -> Report {
    Report { page }
}

#[app(handlers(report, Report))]
pub struct Bot;
# fn main() {}
```

Returning the Props opens the modal. The parameter types (anything implementing
[`ModalValue`](https://docs.rs/composure/latest/composure/command/trait.ModalValue.html)) decide what each input requires, and mismatches are
compile errors:

| Type | Required | Inputs |
|---|---|---|
| `String`, a Choice enum | yes | `<TextInput>`, `<RadioGroup>`, `<StringSelect>` |
| `Option<String>`, `Option<Choice>` | no | the same |
| `bool` | no (checkboxes never are) | `<Checkbox>` |
| `Snowflake`, `User`, `Role`, `PartialChannel` | yes | entity selects |
| [`Attachment`](https://docs.rs/composure/latest/composure/types/struct.Attachment.html) | yes | `<FileUpload>` |
| `Option<..>` of those | no | the same |
| `Vec<..>` of those, of `String`, of a Choice | no; several allowed | selects, `<CheckboxGroup>`, `<FileUpload>` |

Every input needs a parameter and every parameter an input. Inputs in a modal never take
`on:` attributes of their own; the `<Modal>`'s submit reads them all.

A modal can also be submitted to a [`#[modal]`](https://docs.rs/composure/latest/composure/attr.modal.html) handler (`on:submit=rename(id)`)
or answered by `custom_id=..`; inputs then need no matching parameters, and their `name` is the
`custom_id` the handler reads them by.

## Message options

A View is the whole message, not just its components. These set the rest; they work on any
View, `rsx!`'s output or a sub-View's (whose options and files go to the View it is placed in):

| Method | |
|---|---|
| [`.ephemeral()`](https://docs.rs/composure/latest/composure/command/struct.View.html#method.ephemeral) | Only the invoking user sees it. An update keeps the message's ephemerality |
| [`.allowed_mentions(..)`](https://docs.rs/composure/latest/composure/command/struct.View.html#method.allowed_mentions) | Who it may ping ([`AllowedMentions`](https://docs.rs/composure/latest/composure/types/struct.AllowedMentions.html)) |
| [`.suppress_notifications()`](https://docs.rs/composure/latest/composure/command/struct.View.html#method.suppress_notifications) | No push or desktop notification |
| [`.file(..)`](https://docs.rs/composure/latest/composure/command/struct.View.html#method.file) | Uploads a [`File`](https://docs.rs/composure/latest/composure/types/struct.File.html) with the message |

`<File>`, `<Thumbnail>` and `<Item>` take a `File` as their `src` (or `url`): the View uploads it
and points the component at `attachment://<name>`, so the names cannot drift apart.

```rust
use composure::{command, rsx, command::View, types::File};

/// Get your text back as a file
#[command]
fn export(#[option("What to put in the file", max_len = 2000)] text: &str) -> View {
    let file = File::new("export.txt", text.as_bytes()).content_type("text/plain");
    rsx! {
        <Text>"Here's your file:"</Text>
        <File src={file}/>
    }
    .ephemeral()
}
# fn main() {
#     let view = export("hi");
#     assert_eq!(view.files()[0].name, "export.txt");
# }
```

`Ephemeral(view)` still works, and a View with files is sent like any response with files (see
[Files](responses.md#files)).

## Tag reference

Each tag builds the type named in its heading; its attributes are that type's public fields
plus the *special* ones listed. `id` (an `i32`, unique within the message) is accepted by every
tag except `<Modal>`, `<Item>` and `<Option>`, and is omitted below.

### Layout

#### `<Container>`

[`Container`](https://docs.rs/composure/latest/composure/types/struct.Container.html): a frame around other components, like an embed, with an
optional accent bar. Top level only. Children: text, `<Text>`, `<Section>`, `<Gallery>`,
`<File>`, `<Separator>`, `<Row>`, `{..}`.

| Attribute | Type | |
|---|---|---|
| `accent_color` | `u32` (`0xRRGGBB`) | Color of the bar on the left; none when unset |
| `spoiler` | `bool` | Blurs the content until clicked |

```text
<Container accent_color=0x5865F2 spoiler> .. </Container>
```

#### `<Section>`

[`Section`](https://docs.rs/composure/latest/composure/types/struct.Section.html): 1-3 text displays beside an accessory. Children: text and
`<Text>`, plus exactly one `<Button>` or `<Thumbnail>`, which becomes the accessory wherever it
is placed among the children.

| Attribute | Type | |
|---|---|---|
| `accessory` | `impl Into<Component>` | *Special.* The accessory as a value, instead of a child |

```text
<Section>
    <Text>"## Ada Lovelace"</Text>
    <Text>"Mathematician"</Text>
    <Thumbnail url=avatar_url/>
</Section>
```

#### `<Text>` (alias `<TextDisplay>`)

[`TextDisplay`](https://docs.rs/composure/latest/composure/types/struct.TextDisplay.html): markdown. Children: text and `{..}` only. Bare text
outside a `<Text>` makes one too.

| Attribute | Type | |
|---|---|---|
| `content` | `String` | *Special.* The text, instead of children |

#### `<Separator>`

[`Separator`](https://docs.rs/composure/latest/composure/types/struct.Separator.html): vertical space, with a divider line by default. No
children.

| Attribute | Type | |
|---|---|---|
| `small` / `large` | flag | *Special.* Small (default) or large spacing |
| `spacing` | [`SeparatorSpacing`](https://docs.rs/composure/latest/composure/types/enum.SeparatorSpacing.html) | The same, as a value |
| `divider` | `bool` | `false` for space only, no line |

```text
<Separator large divider=false/>
```

#### `<Row>` (alias `<ActionRow>`)

[`ActionRow`](https://docs.rs/composure/latest/composure/types/struct.ActionRow.html): up to 5 `<Button>`s, or one select. Children: those and
`{..}`; no text.

### Content

#### `<Thumbnail>`

[`Thumbnail`](https://docs.rs/composure/latest/composure/types/struct.Thumbnail.html): a small image, as a `<Section>`'s accessory only. No
children.

| Attribute | Type | |
|---|---|---|
| `src` (or `url`) | a URL, a [`File`](https://docs.rs/composure/latest/composure/types/struct.File.html) or an `&`[`Attachment`](https://docs.rs/composure/latest/composure/types/struct.Attachment.html) | *Special, required.* An image URL (or `attachment://<filename>`), a `File` to upload with the message, or an attachment Discord has |
| `description` | `String` | Alt text, max 1024 characters |
| `spoiler` | `bool` | Blurs the image until clicked |

#### `<Gallery>` (alias `<MediaGallery>`)

[`MediaGallery`](https://docs.rs/composure/latest/composure/types/struct.MediaGallery.html): a grid of 1-10 images and videos. Children:
`<Item>`s, and `{..}` of [`MediaGalleryItem`](https://docs.rs/composure/latest/composure/types/struct.MediaGalleryItem.html)s.

#### `<Item>` (alias `<MediaGalleryItem>`)

[`MediaGalleryItem`](https://docs.rs/composure/latest/composure/types/struct.MediaGalleryItem.html): one image or video in a `<Gallery>`. No
children.

| Attribute | Type | |
|---|---|---|
| `src` (or `url`) | a URL, a [`File`](https://docs.rs/composure/latest/composure/types/struct.File.html) or an `&`[`Attachment`](https://docs.rs/composure/latest/composure/types/struct.Attachment.html) | *Special, required.* A URL (or `attachment://<filename>`), a `File` to upload with the message, or an attachment Discord has |
| `description` | `String` | Alt text, max 1024 characters |
| `spoiler` | `bool` | Blurs the media until clicked |

```text
<Gallery>
    <Item url="https://example.com/a.png" description="Before"/>
    <Item url="attachment://after.png" description="After" spoiler/>
    {extra.iter().map(|url| MediaGalleryItem::new(url.as_str()))}
</Gallery>
```

#### `<File>`

[`FileComponent`](https://docs.rs/composure/latest/composure/types/struct.FileComponent.html): a file uploaded with the message, shown inline.
No children. Give it the [`File`](https://docs.rs/composure/latest/composure/types/struct.File.html) and the View uploads it
([Message options](#message-options)).

| Attribute | Type | |
|---|---|---|
| `src` (or `url`) | a [`File`](https://docs.rs/composure/latest/composure/types/struct.File.html), or `impl Into<String>` | *Special, required.* The `File` to upload, or `attachment://<filename>` for one added with [`.file(..)`](https://docs.rs/composure/latest/composure/command/struct.View.html#method.file) |
| `spoiler` | `bool` | Blurs the file until clicked |

### Interactive

#### `<Button>`

[`ButtonComponent`](https://docs.rs/composure/latest/composure/types/struct.ButtonComponent.html): in a `<Row>`, or as a `<Section>`'s
accessory. Children: the label, as text and `{..}`.

| Attribute | Type | |
|---|---|---|
| `primary` / `secondary` / `success` / `danger` | flag | *Special.* The style: blurple, grey (the default), green, red |
| `style` | [`ButtonStyle`](https://docs.rs/composure/latest/composure/types/enum.ButtonStyle.html) | *Special.* The style, as a value |
| `on:click=handler(state..)` | | *Special.* Answered by a `#[component]` handler ([above](#handlers-defined-elsewhere)) |
| `on:click:name=closure` | | *Special.* An [inline handler](#inline-handlers); `#[view]` only |
| `custom_id` | `String` | Answered by whatever handles this `custom_id`; max 100 characters |
| `url` | `String` | A link button (style `Link`), opening the URL; no `on:click` or `custom_id` |
| `sku_id` | [`Snowflake`](https://docs.rs/composure/latest/composure/types/struct.Snowflake.html) | A premium button (style `Premium`) offering the SKU; no label, emoji, `on:click` or `custom_id` |
| `label` | `String` | *Special.* The label, instead of children; max 80 characters |
| `emoji` | `&str` or [`PartialEmoji`](https://docs.rs/composure/latest/composure/types/struct.PartialEmoji.html) | Shown before the label: `"👍"`, or a custom emoji |
| `disabled` | `bool` | Greyed out, not clickable |

A button needs exactly one of `on:click`, `custom_id`, `url` or `sku_id`.

```text
<Row>
    <Button primary on:click:save=move |_| ..>"Save"</Button>
    <Button danger emoji="🗑️" on:click=delete(id)>"Delete"</Button>
    <Button url="https://discord.com/developers/docs">"Docs"</Button>
    <Button custom_id="help" disabled>"Help"</Button>
</Row>
```

#### `<StringSelect>` (alias `<Select>`)

[`StringSelect`](https://docs.rs/composure/latest/composure/types/type.StringSelect.html): a dropdown of up to 25 text options. In a `<Row>`
in messages, or a `<Label>` in modals. Children: `<Option>`s, and `{..}` of
[`SelectOption`](https://docs.rs/composure/latest/composure/types/struct.SelectOption.html)s.

| Attribute | Type | |
|---|---|---|
| `on:select=handler(state..)` | | *Special.* Answered by a `#[component]` handler |
| `on:select:name=closure` | | *Special.* An [inline handler](#selects); `#[view]` only |
| `custom_id` | `String` | Answered by whatever handles this `custom_id` |
| `name` | string literal | *Special, modals only (required there).* The input's `custom_id`, matched to an `on:submit` parameter |
| `options` | a `#[derive(Choice)]` enum | *Special.* One option per variant, before any `<Option>` children |
| `placeholder` | `String` | Shown while nothing is selected; max 150 characters |
| `min` (or `min_values`) | `i32` | Fewest selections, 0-25 (default 1) |
| `max` (or `max_values`) | `i32` | Most selections, 1-25 (default 1; a `Vec` handler parameter makes it every option) |
| `disabled` | `bool` | Not selectable; messages only |
| `required` | `bool` | Modals only; set from the submit parameter's type |

In messages, a select needs `on:select` or `custom_id`.

#### `<Option>`

[`SelectOption`](https://docs.rs/composure/latest/composure/types/struct.SelectOption.html) in a select,
[`GroupOption`](https://docs.rs/composure/latest/composure/types/struct.GroupOption.html) in a `<RadioGroup>` or `<CheckboxGroup>`. Children:
the label, as text and `{..}`.

| Attribute | Type | |
|---|---|---|
| `value` | `impl Into<String>` | *Special, required.* What is submitted when chosen; max 100 characters |
| `label` | `String` | *Special.* The label, instead of children; max 100 characters |
| `description` | `String` | A second line under the label; max 100 characters |
| `emoji` | `&str` or [`PartialEmoji`](https://docs.rs/composure/latest/composure/types/struct.PartialEmoji.html) | Selects only |
| `default` | `bool` | Chosen initially |

```text
<StringSelect on:select:pick=move |_, size: Size| ..>
    <Option value="s" description="Fits most">"Small"</Option>
    <Option value="l" emoji="🐘" default>"Large"</Option>
</StringSelect>
```

#### `<UserSelect>`, `<RoleSelect>`, `<MentionableSelect>`, `<ChannelSelect>`

[`UserSelect`](https://docs.rs/composure/latest/composure/types/type.UserSelect.html), [`RoleSelect`](https://docs.rs/composure/latest/composure/types/type.RoleSelect.html),
[`MentionableSelect`](https://docs.rs/composure/latest/composure/types/type.MentionableSelect.html),
[`ChannelSelect`](https://docs.rs/composure/latest/composure/types/type.ChannelSelect.html): pick users, roles, both, or channels, with
Discord filling in the options. No children. The same attributes as `<StringSelect>` except
`options`, plus:

| Attribute | Type | |
|---|---|---|
| `default_values` | `Vec<`[`SelectDefaultValue`](https://docs.rs/composure/latest/composure/types/struct.SelectDefaultValue.html)`>` | Preselected entities: `vec![SelectDefaultValue::user(id)]` |
| `channel_types` | `Vec<`[`ChannelType`](https://docs.rs/composure/latest/composure/types/enum.ChannelType.html)`>` | `<ChannelSelect>`: the kinds of channel offered |

```text
<Row>
    <ChannelSelect placeholder="Where to post?"
        channel_types={vec![ChannelType::GuildText]}
        on:select:pick=move |_, channel: PartialChannel| ..>
    </ChannelSelect>
</Row>
```

### Modals

#### `<Modal>`

[`ModalCallbackData`](https://docs.rs/composure/latest/composure/types/struct.ModalCallbackData.html), as a
[`ModalView`](https://docs.rs/composure/latest/composure/command/struct.ModalView.html). Must be the markup's only root. Children: text,
`<Text>` and `<Label>`s, at most 5 in total (Discord's limit).

| Attribute | Type | |
|---|---|---|
| `title` | `impl Into<String>` | *Required.* Shown at the top; max 45 characters |
| `on:submit=handler(state..)` | | Submitted to a `#[modal]` handler |
| `on:submit:name=closure` | | An [inline handler](#modal-views); `#[view]` only |
| `custom_id` | `String` | Submitted to whatever handles this `custom_id` |

A `<Modal>` needs exactly one of `on:submit` or `custom_id`, and takes no other attributes.

#### `<Label>`

[`Label`](https://docs.rs/composure/latest/composure/types/struct.Label.html): a title and optional description above one input. Children:
exactly one `<TextInput>`, select, `<RadioGroup>`, `<CheckboxGroup>`, `<Checkbox>` or
`<FileUpload>`.

| Attribute | Type | |
|---|---|---|
| `text` (or `label`) | `impl Into<String>` | *Required.* The title; max 45 characters |
| `description` | `String` | Under the title; max 100 characters |

Every input below takes `name="..."` (*special, required*, a string literal): its `custom_id`,
matched to an `on:submit` parameter. None takes `on:` attributes.

#### `<TextInput>`

[`TextInput`](https://docs.rs/composure/latest/composure/types/struct.TextInput.html): free text. No children.

| Attribute | Type | |
|---|---|---|
| `paragraph` | flag | *Special.* Multi-line; single-line by default |
| `style` | [`TextInputStyle`](https://docs.rs/composure/latest/composure/types/enum.TextInputStyle.html) | *Special.* The same, as a value |
| `min_len` (or `min_length`) | `i32` | 0-4000 |
| `max_len` (or `max_length`) | `i32` | 1-4000 |
| `placeholder` | `String` | Shown while empty; max 100 characters |
| `value` | `String` | Prefilled text; max 4000 characters |
| `required` | `bool` | Set from the submit parameter's type |

#### `<RadioGroup>`

[`RadioGroup`](https://docs.rs/composure/latest/composure/types/struct.RadioGroup.html): pick one of 2-10 options. Children: `<Option>`s, and
`{..}` of [`GroupOption`](https://docs.rs/composure/latest/composure/types/struct.GroupOption.html)s.

| Attribute | Type | |
|---|---|---|
| `options` | a `#[derive(Choice)]` enum | *Special.* One option per variant |
| `required` | `bool` | Set from the submit parameter's type |

#### `<CheckboxGroup>`

[`CheckboxGroup`](https://docs.rs/composure/latest/composure/types/struct.CheckboxGroup.html): pick any of 1-10 options. Children: as
`<RadioGroup>`.

| Attribute | Type | |
|---|---|---|
| `options` | a `#[derive(Choice)]` enum | *Special.* One option per variant |
| `min` (or `min_values`) | `u8` | 0-10 |
| `max` (or `max_values`) | `u8` | 1-10 |
| `required` | `bool` | Set from the submit parameter's type |

#### `<Checkbox>`

[`Checkbox`](https://docs.rs/composure/latest/composure/types/struct.Checkbox.html): one yes/no, submitted as a `bool`. Never required. No
children.

| Attribute | Type | |
|---|---|---|
| `default` | `bool` | Checked initially |

#### `<FileUpload>`

[`FileUpload`](https://docs.rs/composure/latest/composure/types/struct.FileUpload.html): attach files, submitted as
[`Attachment`](https://docs.rs/composure/latest/composure/types/struct.Attachment.html)s. No children.

| Attribute | Type | |
|---|---|---|
| `min` (or `min_values`) | `u8` | 0-10 |
| `max` (or `max_values`) | `u8` | 1-10 (a `Vec` parameter makes it 10) |
| `file_types` | `Vec<String>` | Up to 10 of `"image"`, `"video"`, `"audio"`, or extensions like `".pdf"` |
| `required` | `bool` | Set from the submit parameter's type |

## Styling

Colors are yours: plain `const`s, set where a message is built. A wrapper View per tone keeps
every message alike, and works the same whether a handler answers with it, Background Work
edits with it, or it is sent through REST:

```rust
use composure::{command::View, rsx, view};

pub const SUCCESS: u32 = 0x57f287;

/// Something worked
#[view]
pub fn Success(title: String, children: View) -> View {
    rsx! {
        <Container accent_color=SUCCESS>
            <Text>"### " {title}</Text>
            {children}
        </Container>
    }
}

fn saved(channel: &str) -> View {
    rsx! { <Success title="Saved">"Announcements will appear in " {channel}</Success> }
}
# fn main() { assert_eq!(saved("#news").into_components().unwrap().len(), 1); }
```

Handler errors are the one message Composure builds itself: by default, the ephemeral text of
[`HandlerError::response`](https://docs.rs/composure/latest/composure/command/struct.HandlerError.html#method.response).
`#[app(errors = ..)]` takes an [`ErrorRenderer`](https://docs.rs/composure/latest/composure/command/type.ErrorRenderer.html),
a `fn(&HandlerError) -> InteractionResponse`, to answer them your way, both right away and when
deferred work fails:

```rust
use composure::{
    app, command,
    command::{Ephemeral, HandlerError, IntoResponse},
    rsx,
    types::InteractionResponse,
};

const ERROR: u32 = 0xed4245;

/// Errors as an ephemeral red container
fn render_error(error: &HandlerError) -> InteractionResponse {
    let message = error.user_message().to_string();
    Ephemeral(rsx! { <Container accent_color=ERROR>{message}</Container> }).into_response()
}

/// Ping
#[command]
fn ping() -> &'static str {
    "Pong"
}

#[app(handlers(ping), errors = render_error)]
pub struct Bot;
# fn main() {}
```

[`user_message()`](https://docs.rs/composure/latest/composure/command/struct.HandlerError.html#method.user_message)
is what the user should read: a [`HandlerError::user`](https://docs.rs/composure/latest/composure/command/struct.HandlerError.html#method.user)'s
message, a generic one otherwise. At most one App in a tree sets `errors`; the root uses it for
the whole tree.

## Without `#[view]`

`rsx!` works in any function; only inline handlers need `#[view]`. A plain function returning
`View` is a good fit for layouts without state of their own, and for messages sent through REST.
The message bodies are built from a `&View` with `TryFrom`:
[`CreateMessage`](https://docs.rs/composure/latest/composure/rest/struct.CreateMessage.html), [`CreateFollowup`](https://docs.rs/composure/latest/composure/rest/struct.CreateFollowup.html),
[`EditMessage`](https://docs.rs/composure/latest/composure/rest/struct.EditMessage.html) and [`ExecuteWebhook`](https://docs.rs/composure/latest/composure/rest/struct.ExecuteWebhook.html). Each sets the
components, the Components V2 flag, the View's other flags (ephemeral where the Endpoint takes
it), its allowed mentions and its files, and fails before anything is sent when the View cannot
be (a `custom_id` that could not be built, or past Discord's limits):

```rust
use composure::{
    command::HandlerError,
    rest::{BotClient, CreateMessage, Transport},
    rsx,
    types::Snowflake,
};

async fn announce<T: Transport>(
    bot: &BotClient<T>,
    channel: Snowflake,
    text: &str,
) -> Result<(), HandlerError> {
    let view = rsx! { <Container>"📣 " {text}</Container> };
    bot.create_message(channel, &CreateMessage::try_from(&view)?).await?;
    Ok(())
}
# fn main() {}
```

Built from the same [tone Views](#styling) as your replies, they look alike.
