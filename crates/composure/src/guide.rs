//! Runs the examples in the guide (`book/`, an mdBook) as doctests. Only compiled by
//! `cargo test`, so the published crate doesn't need the book's files.

#[doc = include_str!("../../../book/src/getting_started.md")]
mod getting_started {}

#[doc = include_str!("../../../book/src/commands.md")]
mod commands {}

#[doc = include_str!("../../../book/src/responses.md")]
mod responses {}

#[doc = include_str!("../../../book/src/components.md")]
mod components {}

#[doc = include_str!("../../../book/src/views.md")]
mod views {}

#[doc = include_str!("../../../book/src/theming.md")]
mod theming {}

#[doc = include_str!("../../../book/src/deferring.md")]
mod deferring {}

#[doc = include_str!("../../../book/src/events.md")]
mod events {}

#[doc = include_str!("../../../book/src/platforms.md")]
mod platforms {}

#[doc = include_str!("../../../book/src/testing.md")]
mod testing {}
