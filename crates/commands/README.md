<img src="https://raw.githubusercontent.com/BlueFrog130/composure-rs/main/assets/logo.svg" alt="Composure logo" width="64" height="64" align="right">

# composure_commands

**A builder for Discord application command definitions.**

[![Crates.io](https://img.shields.io/crates/v/composure_commands.svg?logo=rust)](https://crates.io/crates/composure_commands)
[![Docs.rs](https://img.shields.io/docsrs/composure_commands?logo=docs.rs)](https://docs.rs/composure_commands)
[![CI](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/BlueFrog130/composure-rs/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/composure_commands.svg)](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.88+-orange.svg?logo=rust)](https://blog.rust-lang.org/)

> Using [Composure](https://github.com/BlueFrog130/composure-rs)'s macros? You don't need this
> crate: `#[command]` derives each definition from its function, and
> [`composure_sync`](https://crates.io/crates/composure_sync) registers them.

For everything else (commands answered by a hand-written `InteractionHandler`, or definitions
generated from data), build them with the builder:

```rust
use composure::types::Snowflake;
use composure_commands::command::{CommandsBuilder, SubcommandCommandOption};

let commands = CommandsBuilder::new(Snowflake::from_u64(1), None)
    .add_command(|command| {
        command
            .name("todo")
            .description("Manage your list")
            .add_subcommand(|sub| {
                sub.name("add").description("Add an item").add_option(
                    SubcommandCommandOption::new_string_option(
                        "item".into(), "What to add".into(), Some(true), None, None, Some(100), None,
                    ),
                )
            })
    })
    .build();
```

The result is a `Vec<ApplicationCommand>`, ready for `BotClient::overwrite_commands`. The
definition models live in `composure::definition` and are re-exported here.

## License

Licensed under the [Apache License, Version 2.0](https://github.com/BlueFrog130/composure-rs/blob/main/LICENSE).
