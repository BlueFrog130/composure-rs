A View is a message or modal layout written as markup with [`rsx!`](crate::rsx), and
optionally a function of Props with [`#[view]`](crate::view) whose buttons, selects and modal
submits are handled inline. Markup builds the same [`types`](crate::types) a hand-written
response would, so everything here can also be done with
[`Component`](crate::types::Component) constructors; markup is shorter, checks placement at
compile time, and keeps a component next to the code that answers it.

```
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
- [Tag reference](#tag-reference): every tag and its attributes
- [Themes](#themes): where colors come from
- [Without `#[view]`](#without-view): markup in plain functions, sending Views through REST

# Markup

`rsx!` takes JSX-like markup and evaluates to a [`View`](crate::command::View) (a Components V2
message) or, when its only root is `<Modal>`, a [`ModalView`](crate::command::ModalView). Both
implement [`IntoResponse`](crate::command::IntoResponse), so a Command can return them
directly.

```
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

## Tags

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

Discord's limits on counts (40 components in a message, 5 buttons in a row, 25 options in a
select, ...) are not checked; Discord rejects the response.

## Text

String literals are text. Adjacent literals and `{expressions}` join into one
[`TextDisplay`](crate::types::TextDisplay), so these are the same:

```
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

## Expressions

`{expr}` evaluates Rust inside markup. Where components can go, the value is anything
implementing [`IntoChildren`](crate::command::IntoChildren):

| Value | Becomes |
|---|---|
| `&str`, `String`, `&String`, `char`, integers, floats, [`Snowflake`](crate::types::Snowflake), `format_args!(..)` | text, joined with adjacent text |
| [`Component`](crate::types::Component) and every component struct ([`Container`](crate::types::Container), [`ButtonComponent`](crate::types::ButtonComponent), ...) | that component |
| [`View`](crate::command::View), a `#[view]`'s Props | its components, in place |
| `Option<T>` | nothing for `None` |
| `Vec<T>`, arrays, iterator adapters (`map`, `filter`, `filter_map`, `flat_map`, `flatten`, `chain`, `take`, `skip`, `rev`, `once`, `vec::IntoIter`) | each item in order |

So lists are a `map`, and conditionals are an `Option`:

```
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
anything implementing [`IntoText`](crate::command::IntoText): strings, numbers, `Snowflake`, and
`Option`s of them. Call `.to_string()` on other `Display` types.

## Attributes

Attributes are written `name=value`; the value is a Rust expression (a literal, a variable, a
call, a block `{..}`). A bare attribute is `true`: `<Button disabled>` is
`<Button disabled=true>`. Each attribute may appear once.

Apart from the few attributes a tag handles itself (marked *special* in the
[reference](#tag-reference): `url`, `on:click`, the style flags, ...), every attribute sets
the field of the same name on the component's struct. That means:

- every public field of the component's type is an attribute, including `id`
- the value is converted with [`IntoAttr`](crate::command::IntoAttr): a `&str` works for a
  `String` or `Option<String>` field, a `T` for an `Option<T>` field, `"👍"` for an emoji
- an attribute the type does not have is a compile error naming the type
  (`no field `colour` on type `Container``)

Values that contain `<..>` (generics, closures with typed parameters) must be wrapped in
braces, or the markup parser reads `<` as a tag: `options={Vec::<SelectOption>::new()}`.

## Comments

`<!-- comments -->` are allowed anywhere and produce nothing.

# View functions

`#[view]` turns a function returning `View` (or `ModalView`) into a component of its own. The
parameters are its Props, which must be owned types:

```
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
  uses. Setters take anything [`IntoAttr`](crate::command::IntoAttr) converts, and leaving out
  a required Prop is a compile error.
- [`IntoResponse`](crate::command::IntoResponse), [`IntoChildren`](crate::command::IntoChildren),
  [`Rerender`](crate::command::Rerender) for the Props, and the
  [`ViewHandler`](crate::command::ViewHandler) running its inline handlers.

Props:

| Prop | In markup |
|---|---|
| `name: T` | required |
| `name: Option<T>` | optional, `None` when left out; `name=value` sets `Some(value)` |
| `#[prop(default = expr)] name: T` | optional, `expr` when left out |
| `children: View` | the tag's children (see [below](#children)); not usable in handlers |

## Sub-Views

A View is placed in markup as a tag, with its Props as attributes:

```
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

## Children

A `children: View` Prop receives the markup between the tags, which lets a View wrap others:

```
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

# Inline handlers

An `on:<event>:<name>=closure` attribute gives a component its own handler, inside the View that
renders it. The macro turns each into an arm of the View's handler, routed by the `custom_id`
`<prefix>:<name>:<state>`:

- `<prefix>` is the View's name in `snake_case` (`Card` → `card`); change it with
  `#[view(prefix = "c")]` to save characters
- `<name>` is the handler's name, unique within the View
- `<state>` holds the Props the closure uses, found by reading its body (`format!("{name}")`
  counts), encoded with [`custom_id`](crate::command::custom_id)

When the component is used, the Props are decoded and the closure runs with them in scope. Its
return value answers the Interaction:

- a View's Props or a [`View`](crate::command::View) (or a `Result` of one) **re-renders** the
  message the component is on (an [`Update`](crate::command::Update))
- anything else a Command can return ([`CommandOutput`](crate::command::CommandOutput)): a new
  message, `Ephemeral(..)`, a `ModalView` to open a modal, `ctx.defer_update(..)`, ...

| Event | Tags | Closure |
|---|---|---|
| `on:click:name` | `<Button>` | `move \|ctx\| ..` |
| `on:select:name` | selects | `move \|ctx, values: T\| ..` |
| `on:submit:name` | `<Modal>` | `move \|ctx, field: T, ..\| ..` ([Modal Views](#modal-views)) |

`ctx` is a [`ComponentCtx`](crate::command::ComponentCtx) (a
[`ModalCtx`](crate::command::ModalCtx) for submits); write `_` when unused. Untyped, it works with
any State; annotate it to require one, as Commands do:
`{move |ctx: ComponentCtx<impl Platform>| ..}` (braced, because of the `<..>`). Closures may be
`async`.

```
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
  answering with the View fails with a [`HandlerError`](crate::command::HandlerError) (the
  error rides inside the View until then). Keep captured Props small: ids and counters, not
  text.
- Captured Props must implement `serde::Serialize` and `DeserializeOwned`; integers, `bool`,
  `String`, `Snowflake`, `Option`s, tuples, and `#[derive(Serialize, Deserialize)]` types do.
- A `children` Prop cannot be captured: it is markup, not data.
- List every View with inline handlers in an `#[app]`; that is how its `custom_id`s are routed.

## Selects

An `on:select` closure takes the selected values as its second parameter, typed with
[`SelectValue`](crate::command::SelectValue): `String`, `Snowflake`, a resolved
[`User`](crate::types::User), [`Role`](crate::types::Role) or
[`PartialChannel`](crate::types::PartialChannel), or a `#[derive(Choice)]` enum, each alone
(one required selection), in an `Option` (none allowed) or a `Vec` (several; `max` then defaults
to every option).

```
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

## Handlers defined elsewhere

`on:click`, `on:select` and `on:submit` without a name point at a
[`#[component]`](crate::component) or [`#[modal]`](crate::modal) handler instead, with the
state to put in its `custom_id`: `on:click=vote(poll, true)` sets
`custom_id = vote::custom_id(poll, true)`, and `on:click=refresh` calls
`refresh::custom_id()`. These work in any `rsx!`, not only in `#[view]`s. Or set `custom_id=..`
yourself and answer it however you like.

# Modal Views

A `#[view]` returning `ModalView` with a `<Modal>` root is a form. With an inline `on:submit`,
each input's `name` matches one of the closure's parameters, which receive the submitted values
typed:

```
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
[`ModalValue`](crate::command::ModalValue)) decide what each input requires, and mismatches are
compile errors:

| Type | Required | Inputs |
|---|---|---|
| `String`, a Choice enum | yes | `<TextInput>`, `<RadioGroup>`, `<StringSelect>` |
| `Option<String>`, `Option<Choice>` | no | the same |
| `bool` | no (checkboxes never are) | `<Checkbox>` |
| `Snowflake`, `User`, `Role`, `PartialChannel` | yes | entity selects |
| [`Attachment`](crate::types::Attachment) | yes | `<FileUpload>` |
| `Option<..>` of those | no | the same |
| `Vec<..>` of those, of `String`, of a Choice | no; several allowed | selects, `<CheckboxGroup>`, `<FileUpload>` |

Every input needs a parameter and every parameter an input. Inputs in a modal never take
`on:` attributes of their own; the `<Modal>`'s submit reads them all.

A modal can also be submitted to a [`#[modal]`](crate::modal) handler (`on:submit=rename(id)`)
or answered by `custom_id=..`; inputs then need no matching parameters, and their `name` is the
`custom_id` the handler reads them by.

Modals have no color or accent, so [Themes](#themes) do not apply to them.

# Tag reference

Each tag builds the type named in its heading; its attributes are that type's public fields
plus the *special* ones listed. `id` (an `i32`, unique within the message) is accepted by every
tag except `<Modal>`, `<Item>` and `<Option>`, and is omitted below.

## Layout

### `<Container>`

[`Container`](crate::types::Container): a frame around other components, like an embed, with an
optional accent bar. Top level only. Children: text, `<Text>`, `<Section>`, `<Gallery>`,
`<File>`, `<Separator>`, `<Row>`, `{..}`.

| Attribute | Type | |
|---|---|---|
| `accent_color` | `u32` (`0xRRGGBB`) | Color of the bar on the left. Left unset, the App's [Theme](#themes) fills it in |
| `spoiler` | `bool` | Blurs the content until clicked |

```text
<Container accent_color=0x5865F2 spoiler> .. </Container>
```

### `<Section>`

[`Section`](crate::types::Section): 1-3 text displays beside an accessory. Children: text and
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

### `<Text>` (alias `<TextDisplay>`)

[`TextDisplay`](crate::types::TextDisplay): markdown. Children: text and `{..}` only. Bare text
outside a `<Text>` makes one too.

| Attribute | Type | |
|---|---|---|
| `content` | `String` | *Special.* The text, instead of children |

### `<Separator>`

[`Separator`](crate::types::Separator): vertical space, with a divider line by default. No
children.

| Attribute | Type | |
|---|---|---|
| `small` / `large` | flag | *Special.* Small (default) or large spacing |
| `spacing` | [`SeparatorSpacing`](crate::types::SeparatorSpacing) | The same, as a value |
| `divider` | `bool` | `false` for space only, no line |

```text
<Separator large divider=false/>
```

### `<Row>` (alias `<ActionRow>`)

[`ActionRow`](crate::types::ActionRow): up to 5 `<Button>`s, or one select. Children: those and
`{..}`; no text.

## Content

### `<Thumbnail>`

[`Thumbnail`](crate::types::Thumbnail): a small image, as a `<Section>`'s accessory only. No
children.

| Attribute | Type | |
|---|---|---|
| `url` | `impl Into<String>` | *Special, required.* An image URL, or `attachment://<filename>` for a file sent with the message |
| `description` | `String` | Alt text, max 1024 characters |
| `spoiler` | `bool` | Blurs the image until clicked |

### `<Gallery>` (alias `<MediaGallery>`)

[`MediaGallery`](crate::types::MediaGallery): a grid of 1-10 images and videos. Children:
`<Item>`s, and `{..}` of [`MediaGalleryItem`](crate::types::MediaGalleryItem)s.

### `<Item>` (alias `<MediaGalleryItem>`)

[`MediaGalleryItem`](crate::types::MediaGalleryItem): one image or video in a `<Gallery>`. No
children.

| Attribute | Type | |
|---|---|---|
| `url` | `impl Into<String>` | *Special, required.* A URL, or `attachment://<filename>` |
| `description` | `String` | Alt text, max 1024 characters |
| `spoiler` | `bool` | Blurs the media until clicked |

```text
<Gallery>
    <Item url="https://example.com/a.png" description="Before"/>
    <Item url="attachment://after.png" description="After" spoiler/>
    {extra.iter().map(|url| MediaGalleryItem::new(url.as_str()))}
</Gallery>
```

### `<File>`

[`FileComponent`](crate::types::FileComponent): a file uploaded with the message, shown inline.
No children. Attach the file to the response with
[`InteractionResponse::with_file`](crate::types::InteractionResponse::with_file).

| Attribute | Type | |
|---|---|---|
| `url` | `impl Into<String>` | *Special, required.* `attachment://<filename>` |
| `spoiler` | `bool` | Blurs the file until clicked |

## Interactive

### `<Button>`

[`ButtonComponent`](crate::types::ButtonComponent): in a `<Row>`, or as a `<Section>`'s
accessory. Children: the label, as text and `{..}`.

| Attribute | Type | |
|---|---|---|
| `primary` / `secondary` / `success` / `danger` | flag | *Special.* The style: blurple, grey (the default), green, red |
| `style` | [`ButtonStyle`](crate::types::ButtonStyle) | *Special.* The style, as a value |
| `on:click=handler(state..)` | | *Special.* Answered by a `#[component]` handler ([above](#handlers-defined-elsewhere)) |
| `on:click:name=closure` | | *Special.* An [inline handler](#inline-handlers); `#[view]` only |
| `custom_id` | `String` | Answered by whatever handles this `custom_id`; max 100 characters |
| `url` | `String` | A link button (style `Link`), opening the URL; no `on:click` or `custom_id` |
| `sku_id` | [`Snowflake`](crate::types::Snowflake) | A premium button (style `Premium`) offering the SKU; no label, emoji, `on:click` or `custom_id` |
| `label` | `String` | *Special.* The label, instead of children; max 80 characters |
| `emoji` | `&str` or [`PartialEmoji`](crate::types::PartialEmoji) | Shown before the label: `"👍"`, or a custom emoji |
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

### `<StringSelect>` (alias `<Select>`)

[`StringSelect`](crate::types::StringSelect): a dropdown of up to 25 text options. In a `<Row>`
in messages, or a `<Label>` in modals. Children: `<Option>`s, and `{..}` of
[`SelectOption`](crate::types::SelectOption)s.

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

### `<Option>`

[`SelectOption`](crate::types::SelectOption) in a select,
[`GroupOption`](crate::types::GroupOption) in a `<RadioGroup>` or `<CheckboxGroup>`. Children:
the label, as text and `{..}`.

| Attribute | Type | |
|---|---|---|
| `value` | `impl Into<String>` | *Special, required.* What is submitted when chosen; max 100 characters |
| `label` | `String` | *Special.* The label, instead of children; max 100 characters |
| `description` | `String` | A second line under the label; max 100 characters |
| `emoji` | `&str` or [`PartialEmoji`](crate::types::PartialEmoji) | Selects only |
| `default` | `bool` | Chosen initially |

```text
<StringSelect on:select:pick=move |_, size: Size| ..>
    <Option value="s" description="Fits most">"Small"</Option>
    <Option value="l" emoji="🐘" default>"Large"</Option>
</StringSelect>
```

### `<UserSelect>`, `<RoleSelect>`, `<MentionableSelect>`, `<ChannelSelect>`

[`UserSelect`](crate::types::UserSelect), [`RoleSelect`](crate::types::RoleSelect),
[`MentionableSelect`](crate::types::MentionableSelect),
[`ChannelSelect`](crate::types::ChannelSelect): pick users, roles, both, or channels, with
Discord filling in the options. No children. The same attributes as `<StringSelect>` except
`options`, plus:

| Attribute | Type | |
|---|---|---|
| `default_values` | `Vec<`[`SelectDefaultValue`](crate::types::SelectDefaultValue)`>` | Preselected entities: `vec![SelectDefaultValue::user(id)]` |
| `channel_types` | `Vec<`[`ChannelType`](crate::types::ChannelType)`>` | `<ChannelSelect>`: the kinds of channel offered |

```text
<Row>
    <ChannelSelect placeholder="Where to post?"
        channel_types={vec![ChannelType::GuildText]}
        on:select:pick=move |_, channel: PartialChannel| ..>
    </ChannelSelect>
</Row>
```

## Modals

### `<Modal>`

[`ModalCallbackData`](crate::types::ModalCallbackData), as a
[`ModalView`](crate::command::ModalView). Must be the markup's only root. Children: text,
`<Text>` and `<Label>`s, at most 5 in total (Discord's limit).

| Attribute | Type | |
|---|---|---|
| `title` | `impl Into<String>` | *Required.* Shown at the top; max 45 characters |
| `on:submit=handler(state..)` | | Submitted to a `#[modal]` handler |
| `on:submit:name=closure` | | An [inline handler](#modal-views); `#[view]` only |
| `custom_id` | `String` | Submitted to whatever handles this `custom_id` |

A `<Modal>` needs exactly one of `on:submit` or `custom_id`, and takes no other attributes.

### `<Label>`

[`Label`](crate::types::Label): a title and optional description above one input. Children:
exactly one `<TextInput>`, select, `<RadioGroup>`, `<CheckboxGroup>`, `<Checkbox>` or
`<FileUpload>`.

| Attribute | Type | |
|---|---|---|
| `text` (or `label`) | `impl Into<String>` | *Required.* The title; max 45 characters |
| `description` | `String` | Under the title; max 100 characters |

Every input below takes `name="..."` (*special, required*, a string literal): its `custom_id`,
matched to an `on:submit` parameter. None takes `on:` attributes.

### `<TextInput>`

[`TextInput`](crate::types::TextInput): free text. No children.

| Attribute | Type | |
|---|---|---|
| `paragraph` | flag | *Special.* Multi-line; single-line by default |
| `style` | [`TextInputStyle`](crate::types::TextInputStyle) | *Special.* The same, as a value |
| `min_len` (or `min_length`) | `i32` | 0-4000 |
| `max_len` (or `max_length`) | `i32` | 1-4000 |
| `placeholder` | `String` | Shown while empty; max 100 characters |
| `value` | `String` | Prefilled text; max 4000 characters |
| `required` | `bool` | Set from the submit parameter's type |

### `<RadioGroup>`

[`RadioGroup`](crate::types::RadioGroup): pick one of 2-10 options. Children: `<Option>`s, and
`{..}` of [`GroupOption`](crate::types::GroupOption)s.

| Attribute | Type | |
|---|---|---|
| `options` | a `#[derive(Choice)]` enum | *Special.* One option per variant |
| `required` | `bool` | Set from the submit parameter's type |

### `<CheckboxGroup>`

[`CheckboxGroup`](crate::types::CheckboxGroup): pick any of 1-10 options. Children: as
`<RadioGroup>`.

| Attribute | Type | |
|---|---|---|
| `options` | a `#[derive(Choice)]` enum | *Special.* One option per variant |
| `min` (or `min_values`) | `u8` | 0-10 |
| `max` (or `max_values`) | `u8` | 1-10 |
| `required` | `bool` | Set from the submit parameter's type |

### `<Checkbox>`

[`Checkbox`](crate::types::Checkbox): one yes/no, submitted as a `bool`. Never required. No
children.

| Attribute | Type | |
|---|---|---|
| `default` | `bool` | Checked initially |

### `<FileUpload>`

[`FileUpload`](crate::types::FileUpload): attach files, submitted as
[`Attachment`](crate::types::Attachment)s. No children.

| Attribute | Type | |
|---|---|---|
| `min` (or `min_values`) | `u8` | 0-10 |
| `max` (or `max_values`) | `u8` | 1-10 (a `Vec` parameter makes it 10) |
| `file_types` | `Vec<String>` | Up to 10 of `"image"`, `"video"`, `"audio"`, or extensions like `".pdf"` |
| `required` | `bool` | Set from the submit parameter's type |

# Themes

A View answered by a handler goes through the App's [`Theme`](crate::command::Theme) like any
message: every **top-level** `<Container>` without an `accent_color` gets the Theme's
`primary` color, both when sent and when re-rendered by an inline handler. Containers that set
`accent_color` keep it, and [`Unthemed`](crate::command::Unthemed) sends a View as is.

Use the Theme's colors explicitly through `ctx.theme()`, which handlers and Commands have, or by
naming your `Theme` constant:

```
use composure::{rsx, view, command::{Theme, View}};

pub const THEME: Theme = Theme::new().with_primary(0xf5c2e7);

#[view]
pub fn Alert(message: String, failed: bool) -> View {
    let color = if failed { THEME.error } else { THEME.success };
    rsx! { <Container accent_color=color>{message}</Container> }
}
# fn main() {}
```

See [Themes](crate::guide::theming) for everything a Theme fills in.

# Without `#[view]`

`rsx!` works in any function; only inline handlers need `#[view]`. A plain function returning
`View` is a good fit for layouts without state of their own, and for messages sent through REST,
which take components rather than a response:

```
use composure::{rsx, command::{View, custom_id::CustomIdError}, types::Component};

fn announcement(text: &str) -> Result<Vec<Component>, CustomIdError> {
    let view: View = rsx! { <Container>"📣 " {text}</Container> };
    view.into_components()
}
# fn main() { assert_eq!(announcement("hi").unwrap().len(), 1); }
```

Send them with the Components V2 flag
([`MessageFlags::IsComponentsV2`](crate::types::MessageFlags::IsComponentsV2)); REST bodies are
never themed.
