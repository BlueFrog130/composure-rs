# Composure

Yet another Discord bot framework with, but with a specific goal in mind.

The Discord API is moving in a clear direction - interactions. Interactions are an easy way to provide interactivity. Fully featured gateway bots have integrated interactions very well, but still require a dedicated server to be able to run. Thats where Composure comes in.

Composure is a lightweight interaction bot that aims to run on the edge. Everything is moving onto the edge - why not Discord bots? Composure is a frameowrk to parse and respond to interactions.

## Features

- Parse interactions
- Flexible [adapter](#adapters) system

### Adapters

Adapters are responsible for verifying the request, parsing the request body, then responding with an interaction response. Essentially, adapters hook everything together, it is a step above the raw frameowrk.

|Adapters                           |
|-----------------------------------|
|[Cloudflare](./adapters/cloudflare)|

## Todo

- [ ] Explore more adapters
- [ ] Create macro to simplify command tree
