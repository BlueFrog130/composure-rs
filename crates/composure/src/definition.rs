//! [Application command](https://discord.com/developers/docs/interactions/application-commands) definitions:
//! what Discord stores for each command. Used when syncing commands, not when handling interactions.

mod implementation;
mod model;

pub use model::*;
