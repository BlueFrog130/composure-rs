use serde::{Deserialize, Serialize};

/// [Embed Object](https://discord.com/developers/docs/resources/channel#embed-object)
#[derive(Debug, Deserialize, Serialize)]
pub struct Embed {
    /// title of embed
    pub title: Option<String>,

    /// [type of embed](https://discord.com/developers/docs/resources/channel#embed-object-embed-types) (always "rich" for webhook embeds)
    #[serde(rename = "type")]
    pub t: Option<String>,

    /// description of embed
    pub description: Option<String>,

    /// url of embed
    pub url: Option<String>,

    /// timestamp of embed content
    pub timestamp: Option<String>,

    /// color code of the embed
    pub color: Option<u32>,

    /// footer information
    pub footer: Option<EmbedFooter>,

    /// image information
    pub image: Option<EmbedImage>,

    /// thumbnail information
    pub thumbnail: Option<EmbedThumbnail>,

    /// video information
    pub video: Option<EmbedVideo>,

    /// provider information
    pub provider: Option<EmbedProvider>,

    /// author information
    pub author: Option<EmbedAuthor>,

    /// fields information
    pub fields: Option<Vec<EmbedField>>,
}

/// [Embed Footer Structure](https://discord.com/developers/docs/resources/channel#embed-object-embed-footer-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct EmbedFooter {
    /// footer text
    pub text: String,

    /// url of footer icon (only supports http(s) and attachments)
    pub icon_url: Option<String>,

    /// a proxied url of footer icon
    pub proxy_icon_url: Option<String>,
}

/// [Embed Image Structure](https://discord.com/developers/docs/resources/channel#embed-object-embed-image-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct EmbedImage {
    /// source url of image (only supports http(s) and attachments)
    pub url: String,

    /// a proxied url of the image
    pub proxy_url: Option<String>,

    /// height of image
    pub height: Option<i32>,

    /// width of image
    pub width: Option<i32>,
}

/// [Embed Thumbnail Structure](https://discord.com/developers/docs/resources/channel#embed-object-embed-thumbnail-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct EmbedThumbnail {
    /// source url of thumbnail (only supports http(s) and attachments)
    pub url: String,

    /// a proxied url of the thumbnail
    pub proxy_url: Option<String>,

    /// height of thumbnail
    pub height: Option<i32>,

    /// width of thumbnail
    pub width: Option<i32>,
}

/// [Embed Video Structure](https://discord.com/developers/docs/resources/channel#embed-object-embed-video-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct EmbedVideo {
    /// source url of video
    pub url: Option<String>,

    /// a proxied url of the video
    pub proxy_url: Option<String>,

    /// height of video
    pub height: Option<i32>,

    /// width of video
    pub width: Option<i32>,
}

/// [Embed Provider Structure](https://discord.com/developers/docs/resources/channel#embed-object-embed-provider-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct EmbedProvider {
    /// name of provider
    pub name: Option<String>,

    /// url of provider
    pub url: Option<String>,
}

/// [Embed Author Structure](https://discord.com/developers/docs/resources/channel#embed-object-embed-author-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct EmbedAuthor {
    /// name of author
    pub name: String,

    /// url of author (only supports http(s))
    pub url: Option<String>,

    /// url of author icon (only supports http(s) and attachments)
    pub icon_url: Option<String>,

    /// a proxied url of author icon
    pub proxy_icon_url: Option<String>,
}

/// [Embed Field Structure](https://discord.com/developers/docs/resources/channel#embed-object-embed-field-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct EmbedField {
    /// name of the field
    pub name: String,

    /// value of the field
    pub value: String,

    /// whether or not this field should display inline
    pub inline: Option<bool>,
}
