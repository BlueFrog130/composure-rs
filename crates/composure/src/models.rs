use strum::AsRefStr;

mod common;
mod deserialize;
mod serialize;

pub use common::*;
pub use deserialize::*;
pub use serialize::*;

#[derive(Debug, AsRefStr, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Gif,
}
