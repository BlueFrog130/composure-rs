use serde::Deserialize;
use serde_repr::Deserialize_repr;

use crate::{common::Snowflake, deserialize::User};

/// [Application Object](https://discord.comundefinedhttps://discord.com/developers/docs/resources/application#application-object)
#[derive(Debug, Deserialize)]
pub struct Application {
    /// the id of the app
    pub id: Snowflake,

    /// the name of the app
    pub name: String,

    /// the [icon hash](https://discord.com/developers/docs/reference#image-formatting) of the app
    pub icon: Option<String>,

    /// the description of the app
    pub description: String,

    /// an array of rpc origin urls, if rpc is enabled
    pub rpc_origins: Option<Vec<String>>,

    /// when false only app owner can join the app's bot to guilds
    pub bot_public: bool,

    /// when true the app's bot will only join upon completion of the full oauth2 code grant flow
    pub bot_require_code_grant: bool,

    /// the url of the app's terms of service
    pub terms_of_service_url: Option<String>,

    /// the url of the app's privacy policy
    pub privacy_policy_url: Option<String>,

    /// partial user object containing info on the owner of the application
    pub owner: Option<User>,

    /// the hex encoded key for verification in interactions and the GameSDK's [GetTicket](https://discord.com/developers/docs/game-sdk/applications#getticket)
    pub verify_key: String,

    // /// if the application belongs to a team, this will be a list of the members of that team
    // pub team: Option<todo>,
    /// if this application is a game sold on Discord, this field will be the guild to which it has been linked
    pub guild_id: Option<Snowflake>,

    /// if this application is a game sold on Discord, this field will be the id of the "Game SKU" that is created, if exists
    pub primary_sku_id: Option<Snowflake>,

    /// if this application is a game sold on Discord, this field will be the URL slug that links to the store page
    pub slug: Option<String>,

    /// the application's default rich presence invite [cover image hash](https://discord.com/developers/docs/reference#image-formatting)
    pub cover_image: Option<String>,

    /// the application's public [flags](https://discord.com/developers/docs/resources/application#application-object-application-flags)
    pub flags: Option<ApplicationFlags>,

    /// up to 5 tags describing the content and functionality of the application
    pub tags: Option<Vec<String>>,

    /// settings for the application's default in-app authorization link, if enabled
    pub install_params: Option<InstallParams>,

    /// the application's default custom authorization link, if enabled
    pub custom_install_url: Option<String>,

    /// the application's role connection verification entry point, which when configured will render the app as a verification method in the guild role verification configuration
    pub role_connections_verification_url: Option<String>,
}
/// [Install Params Object](https://discord.comundefinedhttps://discord.com/developers/docs/resources/application#install-params-object)
#[derive(Debug, Deserialize)]
pub struct InstallParams {
    /// the [scopes](https://discord.com/developers/docs/topics/oauth2#shared-resources-oauth2-scopes) to add the application to the server with
    pub scopes: Vec<String>,

    /// the [permissions](https://discord.com/developers/docs/topics/permissions) to request for the bot role
    pub permissions: String,
}

/// [Application Flags](https://discord.comundefinedhttps://discord.com/developers/docs/resources/application#application-object-application-flags)
#[derive(Debug, Deserialize_repr)]
#[repr(u32)]
pub enum ApplicationFlags {
    /// Indicates if an app uses the [Auto Moderation API](https://discord.com/developers/docs/resources/auto-moderation)
    ApplicationAutoModerationRuleCreateBadge = 1 << 6,

    /// Intent required for bots in 100 or more servers to receive [presence_update events](https://discord.com/developers/docs/topics/gateway-events#presence-update)
    GatewayPresence = 1 << 12,

    /// Intent required for bots in under 100 servers to receive [presence_update events](https://discord.com/developers/docs/topics/gateway-events#presence-update), found on the Bot page in your app's settings
    GatewayPresenceLimited = 1 << 13,

    /// Intent required for bots in 100 or more servers to receive member-related events like guild_member_add. See the list of member-related events [under GUILD_MEMBERS](https://discord.com/developers/docs/topics/gateway#list-of-intents)
    GatewayGuildMembers = 1 << 14,

    /// Intent required for bots in under 100 servers to receive member-related events like guild_member_add, found on the Bot page in your app's settings. See the list of member-related events [under GUILD_MEMBERS](https://discord.com/developers/docs/topics/gateway#list-of-intents)
    GatewayGuildMembersLimited = 1 << 15,

    /// Indicates unusual growth of an app that prevents verification
    VerificationPendingGuildLimit = 1 << 16,

    /// Indicates if an app is embedded within the Discord client (currently unavailable publicly)
    Embedded = 1 << 17,

    /// Intent required for bots in 100 or more servers to receive [message content](https://support-dev.discord.com/hc/en-us/articles/4404772028055)
    GatewayMessageContent = 1 << 18,

    /// Intent required for bots in under 100 servers to receive [message content](https://support-dev.discord.com/hc/en-us/articles/4404772028055), found on the Bot page in your app's settings
    GatewayMessageContentLimited = 1 << 19,

    /// Indicates if an app has registered global [application commands](https://discord.com/developers/docs/interactions/application-commands)
    ApplicationCommandBadge = 1 << 23,
}
