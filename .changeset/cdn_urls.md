---
composure: minor
---

# CDN URLs for avatars, icons, banners, emojis and stickers

`User::avatar_url`, `default_avatar_url` and `display_avatar_url`, `Member::guild_avatar_url` and `display_avatar_url`, `PartialMember::guild_avatar_url`, `icon_url`/`splash_url`/`discovery_splash_url`/`banner_url` on `Guild`, `PartialGuild` and `GuildPreview`, `Role::icon_url`, `Emoji::url`/`PartialEmoji::url` and `Sticker::url`/`StickerItem::url` build Discord CDN URLs, ready for embed thumbnails. Each image has a `*_url_with(format, size)` variant; animated (`a_`) images default to GIF. `types::image_url` and `CDN_BASE` build any other CDN URL.
