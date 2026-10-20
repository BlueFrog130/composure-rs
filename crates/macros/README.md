# composure_macros

**The macros behind [Composure](https://github.com/BlueFrog130/composure-rs): write a Discord
command as a function, a message as markup.**

[![Crates.io](https://img.shields.io/crates/v/composure_macros.svg?logo=rust)](https://crates.io/crates/composure_macros)
[![Docs.rs](https://img.shields.io/docsrs/composure_macros?logo=docs.rs)](https://docs.rs/composure_macros)
[![CI](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/composure_macros.svg)](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.88+-orange.svg?logo=rust)](https://blog.rust-lang.org/)

> Don't depend on this crate directly: enable `composure`'s `macros` feature, which re-exports
> everything here.
>
> ```sh
> cargo add composure --features macros
> ```

```rust
use composure::{app, command, rsx, view, command::View};

/// Repeat some text
#[command]
fn echo(#[option("What to say", max_len = 200)] text: &str) -> String {
    text.to_string()
}

/// A button that counts its clicks, no database required
#[view]
pub fn Counter(count: u32) -> View {
    rsx! {
        <Text>"Clicked " {count} " times"</Text>
        <Row>
            <Button primary on:click:add=move |_| Counter { count: count + 1 }>"+1"</Button>
        </Row>
    }
}

#[app(handlers(echo, Counter))]
pub struct Bot;
```

| Macro | What it gives you |
|---|---|
| `#[command]` | A slash or context-menu command from a function: Options from its typed parameters, the description from its doc comment |
| `#[group]` | Subcommands: `/mod ban`, `/mod kick` |
| `#[autocomplete]` | Suggestions while the user types |
| `#[derive(Choice)]` | An enum as a fixed set of choices |
| `#[component]`, `#[modal]` | Handlers for buttons, selects and modal submits, with state carried in the `custom_id` |
| `#[derive(Modal)]` | A form whose submit arrives as your struct |
| `rsx!` | JSX-like markup for Components V2 messages and modals |
| `#[view]` | A reusable layout with inline event handlers |
| `#[app]` | Collects everything into the handler an Adapter serves |

Mistakes are caught at compile time with errors that say what to do: a missing description,
an Option bound outside its type, two commands with the same name, a button in the wrong
place, a modal input without a matching parameter.

Learn them in [the guide](https://bluefrog130.github.io/composure-rs/); every
`rsx!` tag and attribute is in the
[Views reference](https://bluefrog130.github.io/composure-rs/views.html#tag-reference).

## License

Licensed under the [Apache License, Version 2.0](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE).
