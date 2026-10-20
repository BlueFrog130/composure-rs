//! A builder for [Command Definitions](https://discord.com/developers/docs/interactions/application-commands),
//! for bots that register Commands without [`#[command]`](https://docs.rs/composure/latest/composure/attr.command.html).
//!
//! With Composure's macros, a Command's definition comes from its function, and
//! [`composure_sync`](https://docs.rs/composure_sync) registers it: prefer that. This crate is
//! for building definitions by hand, e.g. for Commands answered by a custom
//! `InteractionHandler`.
//!
//! ```
//! use composure::types::Snowflake;
//! use composure_commands::command::{CommandsBuilder, SubcommandCommandOption};
//!
//! let commands = CommandsBuilder::new(Snowflake::from_u64(1), None)
//!     .add_command(|command| {
//!         command
//!             .name("todo")
//!             .description("Manage your list")
//!             .add_subcommand(|sub| {
//!                 sub.name("add").description("Add an item").add_option(
//!                     SubcommandCommandOption::new_string_option(
//!                         "item".into(), "What to add".into(), Some(true), None, None, Some(100), None,
//!                     ),
//!                 )
//!             })
//!     })
//!     .build();
//! assert_eq!(commands.len(), 1);
//! ```
//!
//! The definition models themselves live in [`composure::definition`] and are re-exported in
//! [`command`].

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

/// The builders, and the definition models they build
pub mod command;
