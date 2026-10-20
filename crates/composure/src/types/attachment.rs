use serde::{Deserialize, Serialize};

use crate::types::Snowflake;

/// An attachment of a message being sent: which uploaded or existing file it is, and its metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PartialAttachment {
    /// name of file attached
    pub filename: String,

    /// description for the file (max 1024 characters)
    pub description: Option<String>,
}

/// [Attachment Object](https://discord.com/developers/docs/resources/channel#attachment-object)
#[derive(Debug, Clone, Deserialize)]
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

/// A file to upload with a message. Referenced from content/embeds as `attachment://{name}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// file name, including extension
    pub name: String,

    /// alt text (max 1024 characters)
    pub description: Option<String>,

    /// media type; Discord infers it from the name when absent
    pub content_type: Option<String>,

    /// The file's content
    pub bytes: Vec<u8>,
}

impl File {
    /// A file named `name` (with its extension) holding `bytes`
    pub fn new(name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            description: None,
            content_type: None,
            bytes: bytes.into(),
        }
    }

    /// Sets the alt text (max 1024 characters)
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the media type, e.g. `image/png`
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }
}
