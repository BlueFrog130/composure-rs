use serde::{Deserialize, Serialize};

use crate::models::Snowflake;

#[derive(Debug, Deserialize, Serialize)]
pub struct PartialAttachment {
    /// name of file attached
    pub filename: String,

    /// description for the file (max 1024 characters)
    pub description: Option<String>,
}

/// [Attachment Object](https://discord.comundefinedhttps://discord.com/developers/docs/resources/channel#attachment-object)
#[derive(Debug, Deserialize)]
pub struct Attachment {
    /// attachment id
    pub id: Snowflake,

    /// name of file attached
    pub filename: String,

    /// description for the file (max 1024 characters)
    pub description: Option<String>,

    /// the attachment's [media type](https://en.wikipedia.org/wiki/Media_type)
    pub content_type: Option<String>,

    /// size of file in bytes
    pub size: u32,

    /// source url of file
    pub url: String,

    /// a proxied url of file
    pub proxy_url: String,

    /// height of file (if image)
    pub height: Option<u32>,

    /// width of file (if image)
    pub width: Option<u32>,

    /// whether this attachment is ephemeral
    pub ephemeral: Option<bool>,

    /// the duration of the audio file (currently for voice messages)
    pub duration_secs: Option<f32>,

    /// base64 encoded bytearray representing a sampled waveform (currently for voice messages)
    pub waveform: Option<String>,
}
