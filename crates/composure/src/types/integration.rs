use serde_repr::{Deserialize_repr, Serialize_repr};

/// [Interaction Context Types](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-object-interaction-context-types)
///
/// Where an interaction was triggered, or where a command can be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum InteractionContextType {
    /// Within servers
    Guild = 0,

    /// Within DMs with the app's bot user
    BotDm = 1,

    /// Within Group DMs and DMs other than the app's bot user
    PrivateChannel = 2,
}

/// [Application Integration Types](https://discord.com/developers/docs/resources/application#application-object-application-integration-types)
///
/// Where an app can be installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ApplicationIntegrationType {
    /// Installable to servers
    GuildInstall = 0,

    /// Installable to users
    UserInstall = 1,
}
