pub mod auth;
pub mod command;
pub mod definition;
#[cfg(feature = "rest")]
pub mod rest;
pub mod types;

pub use command::Ctx;
#[cfg(feature = "macros")]
pub use composure_macros::{Choice, command, group};

pub trait Mentionable {
    fn to_mention(&self) -> String;
}
