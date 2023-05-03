use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::models::{
    common::{Permissions, Snowflake},
    deserialize::{Member, User},
};

#[derive(Debug, Deserialize)]
pub struct PartialChannel {
    /// the id of this channel
    pub id: Snowflake,

    /// the [type of channel](https://discord.com/developers/docs/resources/channel#channel-object-channel-types)
    #[serde(rename = "type")]
    pub t: ChannelType,

    /// the name of the channel (1-100 characters)
    pub name: Option<String>,

    /// computed permissions for the invoking user in the channel, including overwrites, only included when part of the resolved data received on a slash command interaction
    pub permissions: Option<Permissions>,

    /// thread-specific fields not needed by other channels
    pub thread_metadata: Option<ThreadMetadata>,

    /// for guild channels: id of the parent category for a channel (each parent category can contain up to 50 channels), for threads: id of the text channel this thread was created
    pub parent_id: Option<Snowflake>,
}

/// [Channel Structure](https://discord.com/developers/docs/resources/channel#channel-object-channel-structure)
#[derive(Debug, Deserialize)]
pub struct Channel {
    /// the id of this channel
    pub id: Snowflake,

    /// the [type of channel](https://discord.com/developers/docs/resources/channel#channel-object-channel-types)
    #[serde(rename = "type")]
    pub t: ChannelType,

    /// the id of the guild (may be missing for some channel objects received over gateway guild dispatches)
    pub guild_id: Option<Snowflake>,

    /// sorting position of the channel
    pub position: Option<i32>,

    /// explicit permission overwrites for members and roles
    pub permission_overwrites: Option<Vec<Overwrite>>,

    /// the name of the channel (1-100 characters)
    pub name: Option<String>,

    /// the channel topic (0-4096 characters for GUILD_FORUM channels, 0-1024 characters for all others)
    pub topic: Option<String>,

    /// whether the channel is nsfw
    pub nsfw: Option<bool>,

    /// the id of the last message sent in this channel (or thread for GUILD_FORUM channels) (may not point to an existing or valid message or thread)
    pub last_message_id: Option<Snowflake>,

    /// the bitrate (in bits) of the voice channel
    pub bitrate: Option<u32>,

    /// the user limit of the voice channel
    pub user_limit: Option<u32>,

    /// amount of seconds a user has to wait before sending another message (0-21600); bots, as well as users with the permission manage_messages or manage_channel, are unaffected
    pub rate_limit_per_user: Option<u32>,

    /// the recipients of the DM
    pub recipients: Option<Vec<User>>,

    /// icon hash of the group DM
    pub icon: Option<String>,

    /// id of the creator of the group DM or thread
    pub owner_id: Option<Snowflake>,

    /// application id of the group DM creator if it is bot-created
    pub application_id: Option<Snowflake>,

    /// for group DM channels: whether the channel is managed by an application via the gdm.join OAuth2 scope
    pub managed: Option<bool>,

    /// for guild channels: id of the parent category for a channel (each parent category can contain up to 50 channels), for threads: id of the text channel this thread was created
    pub parent_id: Option<Snowflake>,

    /// when the last pinned message was pinned. This may be null in events such as GUILD_CREATE when a message is not pinned.
    pub last_pin_timestamp: Option<String>,

    /// [voice region](https://discord.com/developers/docs/resources/voice#voice-region-object) id for the voice channel, automatic when set to null
    pub rtc_region: Option<String>,

    /// the camera [video quality mode](https://discord.com/developers/docs/resources/channel#channel-object-video-quality-modes) of the voice channel, 1 when not present
    pub video_quality_mode: Option<VideoQualityMode>,

    /// number of messages (not including the initial message or deleted messages) in a thread.
    pub message_count: Option<u32>,

    /// an approximate count of users in a thread, stops counting at 50
    pub member_count: Option<u8>,

    /// thread-specific fields not needed by other channels
    pub thread_metadata: Option<ThreadMetadata>,

    /// thread member object for the current user, if they have joined the thread, only included on certain API endpoints
    pub member: Option<ThreadMember>,

    /// default duration, copied onto newly created threads, in minutes, threads will stop showing in the channel list after the specified period of inactivity, can be set to: 60, 1440, 4320, 10080
    pub default_auto_archive_duration: Option<u32>,

    /// computed permissions for the invoking user in the channel, including overwrites, only included when part of the resolved data received on a slash command interaction
    pub permissions: Option<Permissions>,

    /// [channel flags](https://discord.com/developers/docs/resources/channel#channel-object-channel-flags) combined as a [bitfield](https://en.wikipedia.org/wiki/Bit_field)
    pub flags: Option<ChannelFlags>,

    /// number of messages ever sent in a thread, it's similar to message_count on message creation, but will not decrement the number when a message is deleted
    pub total_message_sent: Option<u32>,

    /// the set of tags that can be used in a GUILD_FORUM channel
    pub available_tags: Option<Vec<ForumTag>>,

    /// the IDs of the set of tags that have been applied to a thread in a GUILD_FORUM channel
    pub applied_tags: Option<Vec<Snowflake>>,

    /// the emoji to show in the add reaction button on a thread in a GUILD_FORUM channel
    pub default_reaction_emoji: Option<DefaultReaction>,

    /// the initial rate_limit_per_user to set on newly created threads in a channel. this field is copied to the thread at creation time and does not live update.
    pub default_thread_rate_limit_per_user: Option<u32>,

    /// the [default sort order type](https://discord.com/developers/docs/resources/channel#channel-object-sort-order-types) used to order posts in GUILD_FORUM channels. Defaults to null, which indicates a preferred sort order hasn't been set by a channel admin
    pub default_sort_order: Option<SortOrderType>,

    /// the [default forum layout view](https://discord.com/developers/docs/resources/channel#channel-object-forum-layout-types) used to display posts in GUILD_FORUM channels. Defaults to 0, which indicates a layout view has not been set by a channel admin
    pub default_forum_layout: Option<ForumLayoutType>,
}

impl PartialEq for Channel {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// [Channel Types](https://discord.com/developers/docs/resources/channel#channel-object-channel-types)
#[derive(Debug, Deserialize_repr, Serialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum ChannelType {
    /// a text channel within a server
    GuildText = 0,

    /// a direct message between users
    Dm = 1,

    /// a voice channel within a server
    GuildVoice = 2,

    /// a direct message between multiple users
    GroupDm = 3,

    /// an [organizational category](https://support.discord.com/hc/en-us/articles/115001580171-Channel-Categories-101) that contains up to 50 channels
    GuildCategory = 4,

    /// a channel that [users can follow and crosspost into their own server](https://support.discord.com/hc/en-us/articles/360032008192) (formerly news channels)
    GuildAnnouncement = 5,

    /// a temporary sub-channel within a GUILD_ANNOUNCEMENT channel
    AnnouncementThread = 10,

    /// a temporary sub-channel within a GUILD_TEXT or GUILD_FORUM channel
    PublicThread = 11,

    /// a temporary sub-channel within a GUILD_TEXT channel that is only viewable by those invited and those with the MANAGE_THREADS permission
    PrivateThread = 12,

    /// a voice channel for [hosting events with an audience](https://support.discord.com/hc/en-us/articles/1500005513722)
    GuildStageVoice = 13,

    /// the channel in a [hub](https://support.discord.com/hc/en-us/articles/4406046651927-Discord-Student-Hubs-FAQ) containing the listed servers
    GuildDirectory = 14,

    /// Channel that can only contain threads
    GuildForum = 15,
}

/// [Video Quality Modes](https://discord.com/developers/docs/resources/channel#channel-object-video-quality-modes)
#[derive(Debug, Deserialize)]
pub enum VideoQualityMode {
    /// Discord chooses the quality for optimal performance
    Auto = 1,

    /// 720p
    Full = 2,
}

bitflags! {
    /// [Channel Flags](https://discord.com/developers/docs/resources/channel#channel-object-channel-flags)
    #[derive(Debug)]
    pub struct ChannelFlags: u32 {
        /// this thread is pinned to the top of its parent GUILD_FORUM channel
        const Pinned = 1 << 1;

        /// whether a tag is required to be specified when creating a thread in a GUILD_FORUM channel. Tags are specified in the applied_tags field.
        const RequireTag = 1 << 4;
    }
}

impl Serialize for ChannelFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> Deserialize<'de> for ChannelFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bits = u32::deserialize(deserializer)?;
        Ok(ChannelFlags::from_bits_retain(bits))
    }
}

/// [Sort Order Types](https://discord.com/developers/docs/resources/channel#channel-object-sort-order-types)
#[derive(Debug, Deserialize)]
pub enum SortOrderType {
    /// Sort forum posts by activity
    LatestActivity = 0,

    /// Sort forum posts by creation time (from most recent to oldest)
    CreationDate = 1,
}

/// [Forum Layout Types](https://discord.com/developers/docs/resources/channel#channel-object-forum-layout-types)
#[derive(Debug, Deserialize)]
pub enum ForumLayoutType {
    /// No default has been set for forum channel
    NotSet = 0,

    /// Display posts as a list
    ListView = 1,

    /// Display posts as a collection of tiles
    GalleryView = 2,
}

/// [Overwrite Object](https://discord.com/developers/docs/resources/channel#overwrite-object)
#[derive(Debug, Deserialize)]
pub struct Overwrite {
    /// role or user id
    pub id: Snowflake,

    /// either 0 (role) or 1 (member)
    #[serde(rename = "type")]
    pub t: OverwriteType,

    /// permission bit set
    pub allow: Permissions,

    /// permission bit set
    pub deny: Permissions,
}

#[derive(Debug, Deserialize_repr)]
#[repr(u8)]
pub enum OverwriteType {
    Role = 0,
    Member = 1,
}

/// [Thread Metadata Object](https://discord.com/developers/docs/resources/channel#thread-metadata-object)
#[derive(Debug, Deserialize)]
pub struct ThreadMetadata {
    /// whether the thread is archived
    pub archived: bool,

    /// the thread will stop showing in the channel list after auto_archive_duration minutes of inactivity, can be set to: 60, 1440, 4320, 10080
    pub auto_archive_duration: u16,

    /// timestamp when the thread's archive status was last changed, used for calculating recent activity
    pub archive_timestamp: String,

    /// whether the thread is locked; when a thread is locked, only users with MANAGE_THREADS can unarchive it
    pub locked: bool,

    /// whether non-moderators can add other non-moderators to a thread; only available on private threads
    pub invitable: Option<bool>,

    /// timestamp when the thread was created; only populated for threads created after 2022-01-09
    pub create_timestamp: Option<String>,
}

/// [Thread Member Object](https://discord.com/developers/docs/resources/channel#thread-member-object)
#[derive(Debug, Deserialize)]
pub struct ThreadMember {
    /// ID of the thread
    pub id: Option<Snowflake>,

    /// ID of the user
    pub user_id: Option<Snowflake>,

    /// Time the user last joined the thread
    pub join_timestamp: String,

    /// Any user-thread settings, currently only used for notifications
    pub flags: u32,

    /// Additional information about the user
    pub member: Option<Member>,
}

impl PartialEq for ThreadMember {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// [Forum Tag Object](https://discord.com/developers/docs/resources/channel#forum-tag-object)
#[derive(Debug, Deserialize)]
pub struct ForumTag {
    /// the id of the tag
    pub id: Snowflake,

    /// the name of the tag (0-20 characters)
    pub name: String,

    /// whether this tag can only be added to or removed from threads by a member with the MANAGE_THREADS permission
    pub moderated: bool,

    /// the id of a guild's custom emoji *
    pub emoji_id: Option<Snowflake>,

    /// the unicode character of the emoji *
    pub emoji_name: Option<String>,
}

impl PartialEq for ForumTag {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// [Default Reaction Object](https://discord.com/developers/docs/resources/channel#default-reaction-object)
#[derive(Debug, Deserialize)]
pub struct DefaultReaction {
    /// the id of a guild's custom emoji
    pub emoji_id: Option<Snowflake>,

    /// the unicode character of the emoji
    pub emoji_name: Option<String>,
}

impl PartialEq for DefaultReaction {
    fn eq(&self, other: &Self) -> bool {
        self.emoji_id == other.emoji_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn can_deserialize_channel_type() {
        let channel_type_json = "0";

        let channel_type = serde_json::from_str::<ChannelType>(channel_type_json).unwrap();

        assert_eq!(channel_type, ChannelType::GuildText);
    }

    #[test]
    pub fn can_deserialize_channel() {
        let channel_json = r#"{
            "flags": 0,
            "guild_id": "798662131062931547",
            "id": "941169456686723122",
            "last_message_id": "1100155827400229026",
            "name": "bot-stuff",
            "nsfw": false,
            "parent_id": "798662131678969866",
            "permissions": "140737488355327",
            "position": 1,
            "rate_limit_per_user": 0,
            "topic": null,
            "type": 0
        }"#;

        let channel = serde_json::from_str::<Channel>(channel_json);

        let channel = channel.unwrap();

        println!("{:#?}", channel);
    }
}
