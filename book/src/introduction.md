# Composure

Composure is a Rust framework for Discord bots that answer
[Interactions](https://discord.com/developers/docs/interactions/overview) over HTTP, with no
gateway: served from Cloudflare Workers or any server. This guide walks through building one,
chapter by chapter; every example in it is compiled as a test.

1. [Getting started](getting_started.md): crates, features, a first bot, serving and syncing it
2. [Commands](commands.md): `#[command]`, Options, Choices, Groups, context menus,
   autocomplete, Scopes
3. [Responses](responses.md): what handlers return; ephemeral messages, embeds, files, errors
4. [Components and modals](components.md): buttons, selects and forms with `#[component]`,
   `#[modal]` and `#[derive(Modal)]`, state in the `custom_id`
5. [Views](views.md): `rsx!` markup and `#[view]`, with a reference of every tag and attribute
6. [Themes](theming.md): colors and branding on embeds and containers
7. [REST and deferring](deferring.md): calling Discord's API, follow-ups, Deferred Responses
8. [Webhook Events](events.md): app installs, entitlements, lobby messages
9. [Platforms](platforms.md): Adapters, State, platform-specific Commands, logging
10. [Testing](testing.md): running handlers without Discord

The [API documentation](https://docs.rs/composure) covers every type and function.

Terms like *Command*, *Option*, *App*, *Route*, *Scope* and *Sync* are capitalized when they
mean Composure's concept of them; see
[`CONTEXT.md`](https://github.com/BlueFrog130/composure-rs/blob/main/CONTEXT.md) for the
glossary, and [`docs/adr/`](https://github.com/BlueFrog130/composure-rs/tree/main/docs/adr) for
why things are the way they are.
