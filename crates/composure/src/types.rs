//! Discord's data model, shared by Interactions, Responses and REST. Organized by resource;
//! everything is re-exported flat as `composure::types::X`.

use strum::AsRefStr;

mod allowed_mentions;
mod application;
mod attachment;
mod channel;
mod component;
mod embed;
mod emoji;
mod entitlement;
mod event;
mod guild;
mod integration;
mod interaction;
mod interaction_response;
mod member;
mod message;
mod permissions;
mod role;
mod snowflake;
mod sticker;
mod type_field;

pub use allowed_mentions::*;
pub use application::*;
pub use attachment::*;
pub use channel::*;
pub use component::*;
pub use embed::*;
pub use emoji::*;
pub use entitlement::*;
pub use event::*;
pub use guild::*;
pub use integration::*;
pub use interaction::*;
pub use interaction_response::*;
pub use member::*;
pub use message::*;
pub use permissions::*;
pub use role::*;
pub use snowflake::*;
pub use sticker::*;
pub use type_field::*;

#[derive(Debug, AsRefStr, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Gif,
}
