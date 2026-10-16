pub mod auth;
pub mod command;
pub mod definition;
pub mod models;

pub use command::Ctx;
#[cfg(feature = "macros")]
pub use composure_macros::{Choice, command, group};

pub trait Mentionable {
    fn to_mention(&self) -> String;
}
