//! [CDN](https://discord.com/developers/docs/reference#image-formatting) URLs of images: avatars,
//! guild icons, splashes and banners, role icons, emojis and stickers. Each image has a `*_url`
//! method (`None` when the object has no such image) and a `*_url_with(format, size)` variant.

use std::fmt::{Display, Write as _};

use crate::types::{
    Emoji, Guild, GuildPreview, Member, PartialEmoji, PartialGuild, PartialMember, Role, Snowflake,
    Sticker, StickerItem, User,
};

/// Base URL of Discord's CDN
pub const CDN_BASE: &str = "https://cdn.discordapp.com";

/// Where sticker GIFs are served: they don't use [`CDN_BASE`]
const MEDIA_BASE: &str = "https://media.discordapp.net";

/// [Image formats](https://discord.com/developers/docs/reference#image-formatting-image-formats) of CDN images
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    /// `.png`
    Png,
    /// `.jpg`
    Jpeg,
    /// `.webp`; animated for `a_` hashes
    Webp,
    /// `.gif`, for animated images
    Gif,
}

impl ImageFormat {
    /// The file extension, without the dot: `png`, `jpg`, `webp` or `gif`
    pub const fn extension(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Webp => "webp",
            ImageFormat::Gif => "gif",
        }
    }

    /// The default format of the image with this hash: [`Gif`](Self::Gif) for animated (`a_`)
    /// hashes, else [`Png`](Self::Png)
    pub fn of_hash(hash: &str) -> Self {
        if is_animated(hash) {
            ImageFormat::Gif
        } else {
            ImageFormat::Png
        }
    }
}

fn is_animated(hash: &str) -> bool {
    hash.starts_with("a_")
}

/// The URL of the CDN image `hash` under `path` (`avatars/{user_id}`, ...), in `format`
/// ([`ImageFormat::of_hash`] by default) and `size` pixels wide, a power of two from 16 to 4096
/// (Discord's default size when `None`)
pub fn image_url(
    path: impl Display,
    hash: &str,
    format: Option<ImageFormat>,
    size: Option<u16>,
) -> String {
    debug_assert!(
        size.is_none_or(|size| size.is_power_of_two() && (16..=4096).contains(&size)),
        "CDN image sizes are powers of two from 16 to 4096, not {size:?}"
    );
    let format = format.unwrap_or_else(|| ImageFormat::of_hash(hash));
    let mut url = format!("{CDN_BASE}/{path}/{hash}.{}", format.extension());
    let mut separator = '?';
    if let Some(size) = size {
        write!(url, "{separator}size={size}").unwrap();
        separator = '&';
    }
    if format == ImageFormat::Webp && is_animated(hash) {
        write!(url, "{separator}animated=true").unwrap();
    }
    url
}

/// `hash`'s URL under `path`, if there is a hash
fn hashed(
    hash: Option<&str>,
    path: impl Display,
    format: Option<ImageFormat>,
    size: Option<u16>,
) -> Option<String> {
    hash.map(|hash| image_url(path, hash, format, size))
}

impl User {
    /// The avatar's URL; `None` without a custom avatar (see [`display_avatar_url`](Self::display_avatar_url))
    pub fn avatar_url(&self) -> Option<String> {
        self.avatar_url_with(None, None)
    }

    /// [`avatar_url`](Self::avatar_url) in `format` and `size` (see [`image_url`])
    pub fn avatar_url_with(
        &self,
        format: Option<ImageFormat>,
        size: Option<u16>,
    ) -> Option<String> {
        hashed(
            self.avatar.as_deref(),
            format_args!("avatars/{}", self.id),
            format,
            size,
        )
    }

    /// The URL of the default avatar Discord shows for this user without a custom one: one of
    /// six, picked by id (or by discriminator, for users still on the legacy username system).
    /// Always a PNG at a fixed size.
    pub fn default_avatar_url(&self) -> String {
        let index = if self.discriminator == "0" {
            (self.id.to_u64() >> 22) % 6
        } else {
            self.discriminator.parse::<u64>().unwrap_or(0) % 5
        };
        format!("{CDN_BASE}/embed/avatars/{index}.png")
    }

    /// The avatar Discord shows: the custom one's URL, else the default one's
    pub fn display_avatar_url(&self) -> String {
        self.display_avatar_url_with(None, None)
    }

    /// [`display_avatar_url`](Self::display_avatar_url) in `format` and `size` (see
    /// [`image_url`]); the default avatar ignores both
    pub fn display_avatar_url_with(
        &self,
        format: Option<ImageFormat>,
        size: Option<u16>,
    ) -> String {
        self.avatar_url_with(format, size)
            .unwrap_or_else(|| self.default_avatar_url())
    }
}

impl Member {
    /// The URL of the member's avatar in the guild `guild_id`; `None` without one
    pub fn guild_avatar_url(&self, guild_id: Snowflake) -> Option<String> {
        self.guild_avatar_url_with(guild_id, None, None)
    }

    /// [`guild_avatar_url`](Self::guild_avatar_url) in `format` and `size` (see [`image_url`])
    pub fn guild_avatar_url_with(
        &self,
        guild_id: Snowflake,
        format: Option<ImageFormat>,
        size: Option<u16>,
    ) -> Option<String> {
        member_avatar(self.avatar.as_deref(), guild_id, self.user.id, format, size)
    }

    /// The avatar Discord shows in the guild `guild_id`: the member's guild avatar, else the
    /// user's [`display_avatar_url`](User::display_avatar_url)
    pub fn display_avatar_url(&self, guild_id: Snowflake) -> String {
        self.display_avatar_url_with(guild_id, None, None)
    }

    /// [`display_avatar_url`](Self::display_avatar_url) in `format` and `size` (see
    /// [`image_url`]); the default avatar ignores both
    pub fn display_avatar_url_with(
        &self,
        guild_id: Snowflake,
        format: Option<ImageFormat>,
        size: Option<u16>,
    ) -> String {
        self.guild_avatar_url_with(guild_id, format, size)
            .unwrap_or_else(|| self.user.display_avatar_url_with(format, size))
    }
}

impl PartialMember {
    /// The URL of the avatar of the member `user_id` (resolved separately) in the guild
    /// `guild_id`; `None` without one
    pub fn guild_avatar_url(&self, guild_id: Snowflake, user_id: Snowflake) -> Option<String> {
        self.guild_avatar_url_with(guild_id, user_id, None, None)
    }

    /// [`guild_avatar_url`](Self::guild_avatar_url) in `format` and `size` (see [`image_url`])
    pub fn guild_avatar_url_with(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
        format: Option<ImageFormat>,
        size: Option<u16>,
    ) -> Option<String> {
        member_avatar(self.avatar.as_deref(), guild_id, user_id, format, size)
    }
}

fn member_avatar(
    hash: Option<&str>,
    guild_id: Snowflake,
    user_id: Snowflake,
    format: Option<ImageFormat>,
    size: Option<u16>,
) -> Option<String> {
    hashed(
        hash,
        format_args!("guilds/{guild_id}/users/{user_id}/avatars"),
        format,
        size,
    )
}

impl Guild {
    /// The icon's URL; `None` without one
    pub fn icon_url(&self) -> Option<String> {
        self.icon_url_with(None, None)
    }

    /// [`icon_url`](Self::icon_url) in `format` and `size` (see [`image_url`])
    pub fn icon_url_with(&self, format: Option<ImageFormat>, size: Option<u16>) -> Option<String> {
        guild_icon(self.icon.as_deref(), self.id, format, size)
    }

    /// The invite splash's URL; `None` without one
    pub fn splash_url(&self) -> Option<String> {
        self.splash_url_with(None, None)
    }

    /// [`splash_url`](Self::splash_url) in `format` and `size` (see [`image_url`]); no GIF
    pub fn splash_url_with(
        &self,
        format: Option<ImageFormat>,
        size: Option<u16>,
    ) -> Option<String> {
        guild_splash(self.splash.as_deref(), self.id, format, size)
    }

    /// The discovery splash's URL; `None` without one
    pub fn discovery_splash_url(&self) -> Option<String> {
        self.discovery_splash_url_with(None, None)
    }

    /// [`discovery_splash_url`](Self::discovery_splash_url) in `format` and `size` (see
    /// [`image_url`]); no GIF
    pub fn discovery_splash_url_with(
        &self,
        format: Option<ImageFormat>,
        size: Option<u16>,
    ) -> Option<String> {
        guild_discovery_splash(self.discovery_splash.as_deref(), self.id, format, size)
    }

    /// The banner's URL; `None` without one
    pub fn banner_url(&self) -> Option<String> {
        self.banner_url_with(None, None)
    }

    /// [`banner_url`](Self::banner_url) in `format` and `size` (see [`image_url`])
    pub fn banner_url_with(
        &self,
        format: Option<ImageFormat>,
        size: Option<u16>,
    ) -> Option<String> {
        hashed(
            self.banner.as_deref(),
            format_args!("banners/{}", self.id),
            format,
            size,
        )
    }
}

impl PartialGuild {
    /// The icon's URL; `None` without one
    pub fn icon_url(&self) -> Option<String> {
        self.icon_url_with(None, None)
    }

    /// [`icon_url`](Self::icon_url) in `format` and `size` (see [`image_url`])
    pub fn icon_url_with(&self, format: Option<ImageFormat>, size: Option<u16>) -> Option<String> {
        guild_icon(self.icon.as_deref(), self.id, format, size)
    }
}

impl GuildPreview {
    /// The icon's URL; `None` without one
    pub fn icon_url(&self) -> Option<String> {
        self.icon_url_with(None, None)
    }

    /// [`icon_url`](Self::icon_url) in `format` and `size` (see [`image_url`])
    pub fn icon_url_with(&self, format: Option<ImageFormat>, size: Option<u16>) -> Option<String> {
        guild_icon(self.icon.as_deref(), self.id, format, size)
    }

    /// The invite splash's URL; `None` without one
    pub fn splash_url(&self) -> Option<String> {
        self.splash_url_with(None, None)
    }

    /// [`splash_url`](Self::splash_url) in `format` and `size` (see [`image_url`]); no GIF
    pub fn splash_url_with(
        &self,
        format: Option<ImageFormat>,
        size: Option<u16>,
    ) -> Option<String> {
        guild_splash(self.splash.as_deref(), self.id, format, size)
    }

    /// The discovery splash's URL; `None` without one
    pub fn discovery_splash_url(&self) -> Option<String> {
        self.discovery_splash_url_with(None, None)
    }

    /// [`discovery_splash_url`](Self::discovery_splash_url) in `format` and `size` (see
    /// [`image_url`]); no GIF
    pub fn discovery_splash_url_with(
        &self,
        format: Option<ImageFormat>,
        size: Option<u16>,
    ) -> Option<String> {
        guild_discovery_splash(self.discovery_splash.as_deref(), self.id, format, size)
    }
}

fn guild_icon(
    hash: Option<&str>,
    guild_id: Snowflake,
    format: Option<ImageFormat>,
    size: Option<u16>,
) -> Option<String> {
    hashed(hash, format_args!("icons/{guild_id}"), format, size)
}

fn guild_splash(
    hash: Option<&str>,
    guild_id: Snowflake,
    format: Option<ImageFormat>,
    size: Option<u16>,
) -> Option<String> {
    hashed(hash, format_args!("splashes/{guild_id}"), format, size)
}

fn guild_discovery_splash(
    hash: Option<&str>,
    guild_id: Snowflake,
    format: Option<ImageFormat>,
    size: Option<u16>,
) -> Option<String> {
    hashed(
        hash,
        format_args!("discovery-splashes/{guild_id}"),
        format,
        size,
    )
}

impl Role {
    /// The role icon's URL; `None` without one (or with a [`unicode_emoji`](Self::unicode_emoji) instead)
    pub fn icon_url(&self) -> Option<String> {
        self.icon_url_with(None, None)
    }

    /// [`icon_url`](Self::icon_url) in `format` and `size` (see [`image_url`]); no GIF
    pub fn icon_url_with(&self, format: Option<ImageFormat>, size: Option<u16>) -> Option<String> {
        hashed(
            self.icon.as_deref(),
            format_args!("role-icons/{}", self.id),
            format,
            size,
        )
    }
}

impl Emoji {
    /// The custom emoji's URL: a GIF if animated, else a WebP (which Discord recommends for
    /// emojis); `None` for unicode emojis
    pub fn url(&self) -> Option<String> {
        self.url_with(None, None)
    }

    /// [`url`](Self::url) in `format` and `size` (see [`image_url`])
    pub fn url_with(&self, format: Option<ImageFormat>, size: Option<u16>) -> Option<String> {
        emoji(self.id, self.animated, format, size)
    }
}

impl PartialEmoji {
    /// The custom emoji's URL: a GIF if animated, else a WebP (which Discord recommends for
    /// emojis); `None` for unicode emojis
    pub fn url(&self) -> Option<String> {
        self.url_with(None, None)
    }

    /// [`url`](Self::url) in `format` and `size` (see [`image_url`])
    pub fn url_with(&self, format: Option<ImageFormat>, size: Option<u16>) -> Option<String> {
        emoji(self.id, self.animated, format, size)
    }
}

fn emoji(
    id: Option<Snowflake>,
    animated: Option<bool>,
    format: Option<ImageFormat>,
    size: Option<u16>,
) -> Option<String> {
    let format = format.unwrap_or(if animated == Some(true) {
        ImageFormat::Gif
    } else {
        ImageFormat::Webp
    });
    Some(image_url("emojis", &id?.to_string(), Some(format), size))
}

impl Sticker {
    /// The sticker's URL, in its format: PNG (also for APNG), Lottie JSON, or GIF (served from
    /// `media.discordapp.net`). Stickers come in one size.
    pub fn url(&self) -> String {
        sticker(self.id, self.format_type)
    }
}

impl StickerItem {
    /// The sticker's URL, in its format: PNG (also for APNG), Lottie JSON, or GIF (served from
    /// `media.discordapp.net`). Stickers come in one size.
    pub fn url(&self) -> String {
        sticker(self.id, Some(self.format_type))
    }
}

fn sticker(id: Snowflake, format_type: Option<i32>) -> String {
    match format_type {
        Some(3) => format!("{CDN_BASE}/stickers/{id}.json"),
        Some(4) => format!("{MEDIA_BASE}/stickers/{id}.gif"),
        _ => format!("{CDN_BASE}/stickers/{id}.png"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn from<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> T {
        serde_json::from_value(value).unwrap()
    }

    fn user(avatar: Option<&str>, discriminator: &str) -> User {
        from(json!({
            "id": "80351110224678912",
            "username": "nelly",
            "discriminator": discriminator,
            "avatar": avatar,
        }))
    }

    fn member(avatar: Option<&str>, user_avatar: Option<&str>) -> Member {
        from(json!({
            "user": {
                "id": "80351110224678912",
                "username": "nelly",
                "discriminator": "0",
                "avatar": user_avatar,
            },
            "avatar": avatar,
            "roles": [],
            "joined_at": "2015-04-26T06:26:56.936000+00:00",
            "premium_since": null,
            "deaf": false,
            "mute": false,
            "flags": 0,
            "pending": null,
            "permissions": null,
            "communication_disabled_until": null,
        }))
    }

    const GUILD: Snowflake = Snowflake::from_u64(197038439483310086);

    #[test]
    fn image_urls() {
        assert_eq!(
            image_url("avatars/1", "abc", None, None),
            "https://cdn.discordapp.com/avatars/1/abc.png"
        );
        assert_eq!(
            image_url("avatars/1", "a_abc", None, Some(256)),
            "https://cdn.discordapp.com/avatars/1/a_abc.gif?size=256"
        );
        assert_eq!(
            image_url("avatars/1", "a_abc", Some(ImageFormat::Webp), Some(64)),
            "https://cdn.discordapp.com/avatars/1/a_abc.webp?size=64&animated=true"
        );
        assert_eq!(
            image_url("avatars/1", "a_abc", Some(ImageFormat::Webp), None),
            "https://cdn.discordapp.com/avatars/1/a_abc.webp?animated=true"
        );
        assert_eq!(
            image_url("avatars/1", "abc", Some(ImageFormat::Jpeg), None),
            "https://cdn.discordapp.com/avatars/1/abc.jpg"
        );
    }

    #[test]
    #[should_panic(expected = "powers of two")]
    #[cfg(debug_assertions)]
    fn image_sizes_are_checked() {
        image_url("avatars/1", "abc", None, Some(100));
    }

    #[test]
    fn user_avatars() {
        let animated = user(Some("a_8342729096ea3675442027381ff50dfe"), "0");
        assert_eq!(
            animated.avatar_url().unwrap(),
            "https://cdn.discordapp.com/avatars/80351110224678912/a_8342729096ea3675442027381ff50dfe.gif"
        );
        assert_eq!(
            animated
                .avatar_url_with(Some(ImageFormat::Png), Some(128))
                .unwrap(),
            "https://cdn.discordapp.com/avatars/80351110224678912/a_8342729096ea3675442027381ff50dfe.png?size=128"
        );
        assert_eq!(
            animated.display_avatar_url(),
            animated.avatar_url().unwrap()
        );

        let none = user(None, "0");
        assert_eq!(none.avatar_url(), None);
        assert_eq!(none.display_avatar_url(), none.default_avatar_url());
    }

    #[test]
    fn default_avatars() {
        // Unique usernames: (id >> 22) % 6
        assert_eq!(
            user(None, "0").default_avatar_url(),
            format!(
                "https://cdn.discordapp.com/embed/avatars/{}.png",
                (80351110224678912u64 >> 22) % 6
            )
        );
        assert_eq!(
            user(None, "0").default_avatar_url(),
            "https://cdn.discordapp.com/embed/avatars/5.png"
        );
        // Legacy discriminators: discriminator % 5
        assert_eq!(
            user(None, "1337").default_avatar_url(),
            "https://cdn.discordapp.com/embed/avatars/2.png"
        );
    }

    #[test]
    fn member_avatars() {
        let with = member(Some("abc"), Some("def"));
        assert_eq!(
            with.guild_avatar_url(GUILD).unwrap(),
            "https://cdn.discordapp.com/guilds/197038439483310086/users/80351110224678912/avatars/abc.png"
        );
        assert_eq!(
            with.display_avatar_url_with(GUILD, None, Some(32)),
            "https://cdn.discordapp.com/guilds/197038439483310086/users/80351110224678912/avatars/abc.png?size=32"
        );

        let without = member(None, Some("def"));
        assert_eq!(without.guild_avatar_url(GUILD), None);
        assert_eq!(
            without.display_avatar_url(GUILD),
            "https://cdn.discordapp.com/avatars/80351110224678912/def.png"
        );

        let partial: PartialMember = from(json!({
            "nick": null,
            "avatar": "a_abc",
            "roles": [],
            "joined_at": "2015-04-26T06:26:56.936000+00:00",
            "premium_since": null,
            "pending": null,
            "permissions": "0",
        }));
        assert_eq!(
            partial
                .guild_avatar_url(GUILD, Snowflake::from_u64(1))
                .unwrap(),
            "https://cdn.discordapp.com/guilds/197038439483310086/users/1/avatars/a_abc.gif"
        );
    }

    #[test]
    fn guild_images() {
        let guild: Guild = from(json!({
            "id": "197038439483310086", "name": "Testers", "icon": "a_icon",
            "splash": "splash", "discovery_splash": "discovery", "banner": "banner",
            "description": null, "owner_id": "2", "application_id": null, "afk_channel_id": null,
            "afk_timeout": 300, "system_channel_id": null, "system_channel_flags": 0,
            "widget_enabled": false, "widget_channel_id": null, "verification_level": 1,
            "roles": [], "default_message_notifications": 1, "mfa_level": 0,
            "explicit_content_filter": 2, "max_presences": null, "max_members": 500000,
            "vanity_url_code": null, "premium_tier": 0, "premium_subscription_count": 0,
            "preferred_locale": "en-US", "rules_channel_id": null,
            "safety_alerts_channel_id": null, "public_updates_channel_id": null,
            "premium_progress_bar_enabled": false, "nsfw_level": 0, "emojis": [],
            "features": [],
        }));
        assert_eq!(
            guild.icon_url().unwrap(),
            "https://cdn.discordapp.com/icons/197038439483310086/a_icon.gif"
        );
        assert_eq!(
            guild
                .splash_url_with(Some(ImageFormat::Webp), Some(1024))
                .unwrap(),
            "https://cdn.discordapp.com/splashes/197038439483310086/splash.webp?size=1024"
        );
        assert_eq!(
            guild.discovery_splash_url().unwrap(),
            "https://cdn.discordapp.com/discovery-splashes/197038439483310086/discovery.png"
        );
        assert_eq!(
            guild.banner_url().unwrap(),
            "https://cdn.discordapp.com/banners/197038439483310086/banner.png"
        );

        let partial: PartialGuild =
            from(json!({ "id": "197038439483310086", "name": "Testers", "icon": null }));
        assert_eq!(partial.icon_url(), None);

        let preview: GuildPreview = from(json!({
            "id": "197038439483310086",
            "name": "Testers",
            "icon": "icon",
            "splash": null,
            "discovery_splash": "discovery",
            "emojis": [],
            "features": [],
            "approximate_member_count": 1,
            "approximate_presence_count": 1,
            "description": null,
        }));
        assert_eq!(
            preview.icon_url().unwrap(),
            "https://cdn.discordapp.com/icons/197038439483310086/icon.png"
        );
        assert_eq!(preview.splash_url(), None);
        assert_eq!(
            preview.discovery_splash_url().unwrap(),
            "https://cdn.discordapp.com/discovery-splashes/197038439483310086/discovery.png"
        );
    }

    #[test]
    fn role_icons() {
        let role: Role = from(json!({
            "id": "41771983423143936",
            "name": "WE DEM BOYZZ!!!!!!",
            "color": 3447003,
            "hoist": true,
            "icon": "cf3ced8600b777c9486c4ee2cd8d6e1c",
            "unicode_emoji": null,
            "position": 1,
            "permissions": "66321471",
            "managed": false,
            "mentionable": false,
            "flags": 0,
        }));
        assert_eq!(
            role.icon_url().unwrap(),
            "https://cdn.discordapp.com/role-icons/41771983423143936/cf3ced8600b777c9486c4ee2cd8d6e1c.png"
        );
    }

    #[test]
    fn emoji_urls() {
        let custom: Emoji = from(json!({ "id": "41771983429993937", "name": "LUL" }));
        assert_eq!(
            custom.url().unwrap(),
            "https://cdn.discordapp.com/emojis/41771983429993937.webp"
        );
        assert_eq!(
            custom.url_with(Some(ImageFormat::Png), Some(64)).unwrap(),
            "https://cdn.discordapp.com/emojis/41771983429993937.png?size=64"
        );

        let animated: PartialEmoji =
            from(json!({ "id": "41771983429993937", "name": "dance", "animated": true }));
        assert_eq!(
            animated.url().unwrap(),
            "https://cdn.discordapp.com/emojis/41771983429993937.gif"
        );

        let unicode: PartialEmoji = from(json!({ "id": null, "name": "🔥" }));
        assert_eq!(unicode.url(), None);
    }

    #[test]
    fn sticker_urls() {
        let item = |format_type: i32| -> StickerItem {
            from(json!({ "id": "749054660769218631", "name": "Wave", "format_type": format_type }))
        };
        assert_eq!(
            item(1).url(),
            "https://cdn.discordapp.com/stickers/749054660769218631.png"
        );
        assert_eq!(
            item(2).url(),
            "https://cdn.discordapp.com/stickers/749054660769218631.png"
        );
        assert_eq!(
            item(3).url(),
            "https://cdn.discordapp.com/stickers/749054660769218631.json"
        );
        assert_eq!(
            item(4).url(),
            "https://media.discordapp.net/stickers/749054660769218631.gif"
        );

        let sticker: Sticker = from(json!({
            "id": "749054660769218631",
            "name": "Wave",
            "tags": "wave",
            "type": 1,
            "format_type": 3,
        }));
        assert_eq!(
            sticker.url(),
            "https://cdn.discordapp.com/stickers/749054660769218631.json"
        );
    }
}
