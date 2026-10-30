---
cargo-composure: minor
---

# `cargo composure dev`

The new `cargo-composure` crate is the whole development loop for a bot on Workers or a hosted server: it serves the bot, opens a tunnel, points the development application at it, and on every save rebuilds, swaps the new bot in and syncs its Commands to a test server. Settings live in `[package.metadata.composure.dev]`. The application is only changed when `DISCORD_TOKEN` is its token.
