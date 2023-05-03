use serde::{Deserialize, Serialize};

/// [Embed Object](https://discord.com/developers/docs/resources/channel#embed-object)
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename = "rich")]
pub struct Embed {
    /// title of embed
    pub title: Option<String>,

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

impl Embed {
    pub fn new(
        title: Option<String>,
        description: Option<String>,
        url: Option<String>,
        timestamp: Option<String>,
        color: Option<u32>,
        footer: Option<EmbedFooter>,
        image: Option<EmbedImage>,
        thumbnail: Option<EmbedThumbnail>,
        video: Option<EmbedVideo>,
        provider: Option<EmbedProvider>,
        author: Option<EmbedAuthor>,
        fields: Option<Vec<EmbedField>>,
    ) -> Self {
        Self {
            title,
            description,
            url,
            timestamp,
            color,
            footer,
            image,
            thumbnail,
            video,
            provider,
            author,
            fields,
        }
    }
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

impl EmbedFooter {
    pub fn new(text: String, icon_url: Option<String>, proxy_icon_url: Option<String>) -> Self {
        Self {
            text,
            icon_url,
            proxy_icon_url,
        }
    }
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

impl EmbedImage {
    pub fn new(
        url: String,
        proxy_url: Option<String>,
        height: Option<i32>,
        width: Option<i32>,
    ) -> Self {
        Self {
            url,
            proxy_url,
            height,
            width,
        }
    }
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

impl EmbedThumbnail {
    pub fn new(
        url: String,
        proxy_url: Option<String>,
        height: Option<i32>,
        width: Option<i32>,
    ) -> Self {
        Self {
            url,
            proxy_url,
            height,
            width,
        }
    }
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

impl EmbedVideo {
    pub fn new(
        url: Option<String>,
        proxy_url: Option<String>,
        height: Option<i32>,
        width: Option<i32>,
    ) -> Self {
        Self {
            url,
            proxy_url,
            height,
            width,
        }
    }
}

/// [Embed Provider Structure](https://discord.com/developers/docs/resources/channel#embed-object-embed-provider-structure)
#[derive(Debug, Deserialize, Serialize)]
pub struct EmbedProvider {
    /// name of provider
    pub name: Option<String>,

    /// url of provider
    pub url: Option<String>,
}

impl EmbedProvider {
    pub fn new(name: Option<String>, url: Option<String>) -> Self {
        Self { name, url }
    }
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

impl EmbedAuthor {
    pub fn new(
        name: String,
        url: Option<String>,
        icon_url: Option<String>,
        proxy_icon_url: Option<String>,
    ) -> Self {
        Self {
            name,
            url,
            icon_url,
            proxy_icon_url,
        }
    }
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

impl EmbedField {
    pub fn new(name: String, value: String, inline: Option<bool>) -> Self {
        Self {
            name,
            value,
            inline,
        }
    }
}

/// Builder for Embeds
pub struct EmbedBuilder {
    embed: Embed,
}

impl EmbedBuilder {
    pub fn new() -> Self {
        EmbedBuilder {
            embed: Embed::new(
                None, None, None, None, None, None, None, None, None, None, None, None,
            ),
        }
    }

    pub fn with_title(mut self, title: String) -> Self {
        self.embed.title = Some(title);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.embed.description = Some(description);
        self
    }

    pub fn with_url(mut self, url: String) -> Self {
        self.embed.url = Some(url);
        self
    }

    pub fn with_timestamp(mut self, timestamp: String) -> Self {
        self.embed.timestamp = Some(timestamp);
        self
    }

    pub fn with_color(mut self, color: u32) -> Self {
        self.embed.color = Some(color);
        self
    }

    pub fn with_footer(mut self, footer: EmbedFooter) -> Self {
        self.embed.footer = Some(footer);
        self
    }

    pub fn with_image(mut self, image: EmbedImage) -> Self {
        self.embed.image = Some(image);
        self
    }

    pub fn with_thumbnail(mut self, thumbnail: EmbedThumbnail) -> Self {
        self.embed.thumbnail = Some(thumbnail);
        self
    }

    pub fn with_video(mut self, video: EmbedVideo) -> Self {
        self.embed.video = Some(video);
        self
    }

    pub fn with_provider(mut self, provider: EmbedProvider) -> Self {
        self.embed.provider = Some(provider);
        self
    }

    pub fn with_author(mut self, author: EmbedAuthor) -> Self {
        self.embed.author = Some(author);
        self
    }

    pub fn with_fields(mut self, fields: Vec<EmbedField>) -> Self {
        self.embed.fields = Some(fields);
        self
    }

    pub fn build(self) -> Embed {
        self.embed
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn embed_serialize_test() {
        let embed = EmbedBuilder::new()
            .with_title("title".to_string())
            .with_color(0xFFFFFF)
            .build();

        let json = serde_json::to_string_pretty(&embed).unwrap();

        println!("{}", json);
    }

    #[test]
    pub fn embed_deserialize_test() {
        let json = r#"{
            "type": "rich",
            "title": "title",
            "description": null,
            "url": null,
            "timestamp": null,
            "color": null,
            "footer": null,
            "image": null,
            "thumbnail": null,
            "video": null,
            "provider": null,
            "author": null,
            "fields": null
          }"#;

        let embed = serde_json::from_str::<Embed>(json).unwrap();

        println!("{:#?}", embed);
    }
}
