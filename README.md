# Composure

Yet another Discord bot framework with, but with a specific goal in mind.

The Discord API is moving in a clear direction - interactions. Interactions are an easy way to provide interactivity. Fully featured gateway bots have integrated interactions very well, but still require a dedicated server to be able to run. Thats where Composure comes in.

Composure is a lightweight interaction bot that aims to run on the edge. Everything is moving onto the edge - why not Discord bots? Composure is a frameowrk to parse and respond to interactions.

## Features

- Parse interactions
- Define each command once with `#[command]`: options, description, and response are inferred from the function
- Sync commands with Discord from the same definitions (`composure_sync::sync_from_env`)
- Flexible [adapter](#adapters) system

```rust,ignore
use composure::{app, command};

/// Repeat some text
#[command]
fn echo(
    #[option("What to say", max_len = 200)] text: &str,
    #[option("How many times", min = 1, max = 5)] times: Option<u8>,
) -> String {
    text.repeat(times.unwrap_or(1).into())
}

/// Everything the bot serves: Commands, Groups, component and modal handlers, Views
#[app(handlers(echo))]
pub struct Bot;
```

`Bot` is handed to an adapter to serve interactions, and to `composure_sync::sync_from_env` in a host-only bin to sync the command definitions. See [the example bot](./examples/bot), served on [Cloudflare Workers](./examples/cloudflare) and on [a tokio server](./examples/tokio).

### Adapters

Adapters are responsible for verifying the request, parsing the request body, then responding with an interaction response. Essentially, adapters hook everything together, it is a step above the raw frameowrk.

|Adapters                           |
|-----------------------------------|
|[Cloudflare](./crates/adapters/cloudflare)|
|[tokio (hosted server)](./crates/adapters/tokio)|

## Todo

- [ ] Explore more adapters
- [x] Create macro to simplify command tree
- [ ] Autocomplete, components, and modals through `#[command]`
- [ ] Deferred responses and follow-ups
- [ ] Command localizations
- [ ] Handle message component interaction
- [ ] Handle autocomplete interaction
- [ ] Handle modal submit interaction
