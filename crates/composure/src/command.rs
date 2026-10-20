//! Commands defined once with `#[command]`, from which both their handler and their Command
//! Definition derive, collected with the handlers for components, modals and Views into an App
//! with `#[app]`.
//!
//! ```ignore
//! use composure::{app, command};
//!
//! /// Repeat some text
//! #[command]
//! fn echo(
//!     #[option("What to say", max_len = 200)] text: &str,
//!     #[option("How many times", min = 1, max = 5)] times: Option<u8>,
//! ) -> String {
//!     text.repeat(times.unwrap_or(1).into())
//! }
//!
//! #[app(handlers(echo))]
//! pub struct Bot;
//! ```

mod autocomplete;
mod component;
mod ctx;
pub mod custom_id;
mod data;
mod error;
mod event;
mod handler;
mod modal;
mod option;
#[cfg(feature = "rest")]
mod platform;
mod response;
mod route;
mod set;
mod theme;
mod view;

pub use autocomplete::*;
pub use component::*;
pub use ctx::*;
pub use data::*;
pub use error::*;
pub use event::*;
pub use handler::*;
pub use modal::*;
pub use option::*;
#[cfg(feature = "rest")]
pub use platform::*;
pub use response::*;
pub use route::*;
pub use set::*;
pub use theme::*;
pub use view::*;
