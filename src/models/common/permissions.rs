use bitflags::bitflags;
use serde::{Deserialize, Serialize};

bitflags! {
    /// [Bitwise Permission Flags](https://discord.comundefinedhttps://discord.com/developers/docs/topics/permissions#permissions-bitwise-permission-flags)
    #[derive(Debug)]
    pub struct Permissions: u64 {
        /// Allows creation of instant invites
        const CreateInstantInvite = (1 << 0);

        /// Allows kicking members
        const KickMembers = (1 << 1);

        /// Allows banning members
        const BanMembers = (1 << 2);

        /// Allows all permissions and bypasses channel permission overwrites
        const Administrator = (1 << 3);

        /// Allows management and editing of channels
        const ManageChannels = (1 << 4);

        /// Allows management and editing of the guild
        const ManageGuild = (1 << 5);

        /// Allows for the addition of reactions to messages
        const AddReactions = (1 << 6);

        /// Allows for viewing of audit logs
        const ViewAuditLog = (1 << 7);

        /// Allows for using priority speaker in a voice channel
        const PrioritySpeaker = (1 << 8);

        /// Allows the user to go live
        const Stream = (1 << 9);

        /// Allows guild members to view a channel, which includes reading messages in text channels and joining voice channels
        const ViewChannel = (1 << 10);

        /// Allows for sending messages in a channel and creating threads in a forum (does not allow sending messages in threads)
        const SendMessages = (1 << 11);

        /// Allows for sending of /tts messages
        const SendTtsMessages = (1 << 12);

        /// Allows for deletion of other users messages
        const ManageMessages = (1 << 13);

        /// Links sent by users with this permission will be auto-embedded
        const EmbedLinks = (1 << 14);

        /// Allows for uploading images and files
        const AttachFiles = (1 << 15);

        /// Allows for reading of message history
        const ReadMessageHistory = (1 << 16);

        /// Allows for using the @everyone tag to notify all users in a channel, and the @here tag to notify all online users in a channel
        const MentionEveryone = (1 << 17);

        /// Allows the usage of custom emojis from other servers
        const UseExternalEmojis = (1 << 18);

        /// Allows for viewing guild insights
        const ViewGuildInsights = (1 << 19);

        /// Allows for joining of a voice channel
        const Connect = (1 << 20);

        /// Allows for speaking in a voice channel
        const Speak = (1 << 21);

        /// Allows for muting members in a voice channel
        const MuteMembers = (1 << 22);

        /// Allows for deafening of members in a voice channel
        const DeafenMembers = (1 << 23);

        /// Allows for moving of members between voice channels
        const MoveMembers = (1 << 24);

        /// Allows for using voice-activity-detection in a voice channel
        const UseVad = (1 << 25);

        /// Allows for modification of own nickname
        const ChangeNickname = (1 << 26);

        /// Allows for modification of other users nicknames
        const ManageNicknames = (1 << 27);

        /// Allows management and editing of roles
        const ManageRoles = (1 << 28);

        /// Allows management and editing of webhooks
        const ManageWebhooks = (1 << 29);

        /// Allows management and editing of emojis, stickers, and soundboard sounds
        const ManageGuildExpressions = (1 << 30);

        /// Allows members to use application commands, including slash commands and context menu commands.
        const UseApplicationCommands = (1 << 31);

        /// Allows for requesting to speak in stage channels. (This permission is under active development and may be changed or removed.)
        const RequestToSpeak = (1 << 32);

        /// Allows for creating, editing, and deleting scheduled events
        const ManageEvents = (1 << 33);

        /// Allows for deleting and archiving threads, and viewing all private threads
        const ManageThreads = (1 << 34);

        /// Allows for creating public and announcement threads
        const CreatePublicThreads = (1 << 35);

        /// Allows for creating private threads
        const CreatePrivateThreads = (1 << 36);

        /// Allows the usage of custom stickers from other servers
        const UseExternalStickers = (1 << 37);

        /// Allows for sending messages in threads
        const SendMessagesInThreads = (1 << 38);

        /// Allows for using Activities (applications with the EMBEDDED flag) in a voice channel
        const UseEmbeddedActivities = (1 << 39);

        /// Allows for timing out users to prevent them from sending or reacting to messages in chat and threads, and from speaking in voice and stage channels
        const ModerateMembers = (1 << 40);

        /// Allows for viewing role subscription insights
        const ViewCreatorMonetizationAnalytics = (1 << 41);

        /// Allows for using soundboard in a voice channel
        const UseSoundboard = (1 << 42);

        /// Allows sending voice messages
        const SendVoiceMessages = (1 << 46);
    }
}

impl Serialize for Permissions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.bits().to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bit_str = String::deserialize(deserializer)?;
        let bits = bit_str
            .parse::<u64>()
            .map_err(|e| serde::de::Error::custom(e))?;

        // Permissions::from_bits(bits).ok_or(serde::de::Error::custom("Unexpected permissions flags"))
        Ok(Permissions::from_bits_retain(bits))
    }
}

#[cfg(test)]
pub mod tests {
    use super::Permissions;

    #[test]
    pub fn serialize() {
        let flags = Permissions::UseApplicationCommands | Permissions::UseSoundboard;
        println!("{}", serde_json::to_string_pretty(&flags).unwrap());
    }

    #[test]
    pub fn deserialize() {
        let json = r#""66321471""#;
        let permissions: Permissions = serde_json::from_str(json).unwrap();
        println!("{:#?}", permissions);
    }

    #[test]
    pub fn deserialize_this() {
        let json = r#""140737488355327""#;
        let permissions: Permissions = serde_json::from_str(json).unwrap();
        println!("{:#?}", permissions);
    }
}
