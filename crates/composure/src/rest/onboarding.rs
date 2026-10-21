//! [Guild onboarding](https://discord.com/developers/docs/resources/guild#get-guild-onboarding)
//! and the [welcome screen](https://discord.com/developers/docs/resources/guild#get-guild-welcome-screen)

use serde::Serialize;

use crate::types::{
    GuildOnboarding, OnboardingMode, OnboardingPromptType, Snowflake, WelcomeScreen,
    WelcomeScreenChannel,
};

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    GET_GUILD_ONBOARDING = Get "/guilds/{guild_id}/onboarding";
    MODIFY_GUILD_ONBOARDING = Put "/guilds/{guild_id}/onboarding";
    GET_GUILD_WELCOME_SCREEN = Get "/guilds/{guild_id}/welcome-screen";
    MODIFY_GUILD_WELCOME_SCREEN = Patch "/guilds/{guild_id}/welcome-screen";
}

/// Body of [`BotClient::modify_guild_onboarding`]. Absent fields are unchanged.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModifyOnboarding<'a> {
    /// Every question, replacing the current ones (at most 15)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<&'a [OnboardingPromptFields<'a>]>,

    /// Channels every new member is opted into (at most 500)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_channel_ids: Option<&'a [Snowflake]>,

    /// Whether onboarding is on
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Which criteria count toward the onboarding requirements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<OnboardingMode>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// One question in [`ModifyOnboarding::prompts`]
#[derive(Debug, Clone, Copy, Serialize)]
pub struct OnboardingPromptFields<'a> {
    /// The prompt's id: an existing one to keep it, any new snowflake for a new prompt
    pub id: Snowflake,

    /// The question, 1-100 characters
    pub title: &'a str,

    /// The answers (at most 50)
    pub options: &'a [OnboardingPromptOptionFields<'a>],

    /// How the options are shown
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub t: Option<OnboardingPromptType>,

    /// Whether only one option may be picked
    #[serde(skip_serializing_if = "Option::is_none")]
    pub single_select: Option<bool>,

    /// Whether it must be answered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Whether it is asked during onboarding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_onboarding: Option<bool>,
}

/// One answer in [`OnboardingPromptFields::options`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct OnboardingPromptOptionFields<'a> {
    /// An existing option's id to keep it; none for a new option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Snowflake>,

    /// The answer, 1-50 characters
    pub title: &'a str,

    /// Shown under the answer, up to 100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,

    /// Roles the member gets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_ids: Option<&'a [Snowflake]>,

    /// Channels the member is opted into
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_ids: Option<&'a [Snowflake]>,

    /// A custom emoji shown with it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<Snowflake>,

    /// A unicode emoji (or the custom emoji's name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<&'a str>,

    /// Whether the custom emoji is animated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_animated: Option<bool>,
}

/// Body of [`BotClient::modify_guild_welcome_screen`]. Absent fields are unchanged; `Some(None)`
/// clears.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModifyWelcomeScreen<'a> {
    /// Whether the welcome screen is shown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<Option<bool>>,

    /// Up to 5 channels suggested to new members
    #[serde(skip_serializing_if = "Option::is_none")]
    pub welcome_channels: Option<Option<&'a [WelcomeScreenChannel]>>,

    /// The server description shown, up to 140 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<&'a str>>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

impl<T: Transport> BotClient<T> {
    /// [Get Guild Onboarding](https://discord.com/developers/docs/resources/guild#get-guild-onboarding)
    pub async fn get_guild_onboarding(
        &self,
        guild_id: Snowflake,
    ) -> Result<Response<GuildOnboarding>> {
        let url = self.rest.url(GET_GUILD_ONBOARDING, &[&guild_id]);
        self.rest
            .call(GET_GUILD_ONBOARDING, url, Body::NONE, None)
            .await
    }

    /// [Modify Guild Onboarding](https://discord.com/developers/docs/resources/guild#modify-guild-onboarding)
    pub async fn modify_guild_onboarding(
        &self,
        guild_id: Snowflake,
        onboarding: &ModifyOnboarding<'_>,
    ) -> Result<Response<GuildOnboarding>> {
        let url = self.rest.url(MODIFY_GUILD_ONBOARDING, &[&guild_id]);
        self.rest
            .call(
                MODIFY_GUILD_ONBOARDING,
                url,
                Body::Json(onboarding),
                onboarding.reason,
            )
            .await
    }

    /// [Get Guild Welcome Screen](https://discord.com/developers/docs/resources/guild#get-guild-welcome-screen)
    pub async fn get_guild_welcome_screen(
        &self,
        guild_id: Snowflake,
    ) -> Result<Response<WelcomeScreen>> {
        let url = self.rest.url(GET_GUILD_WELCOME_SCREEN, &[&guild_id]);
        self.rest
            .call(GET_GUILD_WELCOME_SCREEN, url, Body::NONE, None)
            .await
    }

    /// [Modify Guild Welcome Screen](https://discord.com/developers/docs/resources/guild#modify-guild-welcome-screen)
    pub async fn modify_guild_welcome_screen(
        &self,
        guild_id: Snowflake,
        screen: &ModifyWelcomeScreen<'_>,
    ) -> Result<Response<WelcomeScreen>> {
        let url = self.rest.url(MODIFY_GUILD_WELCOME_SCREEN, &[&guild_id]);
        self.rest
            .call(
                MODIFY_GUILD_WELCOME_SCREEN,
                url,
                Body::Json(screen),
                screen.reason,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::json;

    use super::super::{Method::*, testing::MockTransport};
    use super::*;

    #[test]
    fn routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        let g = Snowflake::from_u64(5);
        let roles = [Snowflake::from_u64(7)];
        let options = [OnboardingPromptOptionFields {
            title: "Rust",
            role_ids: Some(&roles),
            emoji_name: Some("🦀"),
            ..Default::default()
        }];
        let prompts = [OnboardingPromptFields {
            id: Snowflake::from_u64(9),
            title: "What do you write?",
            options: &options,
            t: Some(OnboardingPromptType::Dropdown),
            single_select: None,
            required: Some(true),
            in_onboarding: None,
        }];
        let onboarding = ModifyOnboarding {
            prompts: Some(&prompts),
            enabled: Some(true),
            ..Default::default()
        };
        let channels = [WelcomeScreenChannel {
            channel_id: Snowflake::from_u64(8),
            description: "Say hi".into(),
            emoji_id: None,
            emoji_name: Some("👋".into()),
        }];
        let screen = ModifyWelcomeScreen {
            welcome_channels: Some(Some(&channels)),
            description: Some(None),
            ..Default::default()
        };
        block_on(c.get_guild_onboarding(g)).unwrap();
        block_on(c.modify_guild_onboarding(g, &onboarding)).unwrap();
        block_on(c.get_guild_welcome_screen(g)).unwrap();
        block_on(c.modify_guild_welcome_screen(g, &screen)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/guilds/5/onboarding".into()),
                (Put, "/guilds/5/onboarding".into()),
                (Get, "/guilds/5/welcome-screen".into()),
                (Patch, "/guilds/5/welcome-screen".into()),
            ]
        );
        assert_eq!(
            t.body(1),
            json!({
                "prompts": [{ "id": "9", "title": "What do you write?", "type": 1, "required": true,
                              "options": [{ "title": "Rust", "role_ids": ["7"], "emoji_name": "🦀" }] }],
                "enabled": true,
            })
        );
        assert_eq!(
            t.body(3),
            json!({
                "welcome_channels": [{ "channel_id": "8", "description": "Say hi", "emoji_id": null, "emoji_name": "👋" }],
                "description": null,
            })
        );
    }
}
