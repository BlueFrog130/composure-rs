use strum::AsRefStr;

mod common;
mod deserialize;
mod serialize;

pub use common::*;
pub use deserialize::*;
pub use serialize::*;

const DISCORD_CDN: &str = "https://cdn.discordapp.com";

#[derive(Debug, AsRefStr, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Webp,
    Gif,
}

trait Avatar {
    fn get_cdn_url() -> &'static str {
        DISCORD_CDN
    }
    fn get_avatar_url(&self, preferred_format: ImageFormat) -> Option<String>;
}
