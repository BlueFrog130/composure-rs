//! [Guild onboarding](https://discord.com/developers/docs/resources/guild#guild-onboarding-object)
//! and the [welcome screen](https://discord.com/developers/docs/resources/guild#welcome-screen-object):
//! what new members are asked and shown

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::{PartialEmoji, Snowflake};

/// [Guild Onboarding](https://discord.com/developers/docs/resources/guild#guild-onboarding-object)
#[derive(Debug, Clone, Deserialize)]
pub struct GuildOnboarding {
    /// The guild
    pub guild_id: Snowflake,

    /// The questions new members answer
    pub prompts: Vec<OnboardingPrompt>,

    /// Channels every new member is opted into
    pub default_channel_ids: Vec<Snowflake>,

    /// Whether onboarding is on
    pub enabled: bool,

    /// Which criteria count toward the onboarding requirements
    pub mode: OnboardingMode,
}

/// [Onboarding Prompt](https://discord.com/developers/docs/resources/guild#guild-onboarding-object-onboarding-prompt-structure):
/// one question
#[derive(Debug, Clone, Deserialize)]
pub struct OnboardingPrompt {
    /// The prompt's id
    pub id: Snowflake,

    /// How the options are shown
    #[serde(rename = "type")]
    pub t: OnboardingPromptType,

    /// The answers to pick from
    pub options: Vec<OnboardingPromptOption>,

    /// The question
    pub title: String,

    /// Whether only one option may be picked
    pub single_select: bool,

    /// Whether it must be answered
    pub required: bool,

    /// Whether it is asked during onboarding, not only in Channels & Roles later
    pub in_onboarding: bool,
}

/// [Prompt Option](https://discord.com/developers/docs/resources/guild#guild-onboarding-object-prompt-option-structure):
/// one answer, and what picking it gives
#[derive(Debug, Clone, Deserialize)]
pub struct OnboardingPromptOption {
    /// The option's id
    pub id: Snowflake,

    /// Channels the member is opted into
    pub channel_ids: Vec<Snowflake>,

    /// Roles the member gets
    pub role_ids: Vec<Snowflake>,

    /// The emoji shown with it
    pub emoji: Option<PartialEmoji>,

    /// The answer
    pub title: String,

    /// Shown under the answer
    pub description: Option<String>,
}

/// [Onboarding Mode](https://discord.com/developers/docs/resources/guild#guild-onboarding-object-onboarding-mode)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum OnboardingMode {
    /// Only default channels count toward the requirements
    Default = 0,
    /// Default channels and questions count toward the requirements
    Advanced = 1,
}

/// [Prompt Type](https://discord.com/developers/docs/resources/guild#guild-onboarding-object-prompt-types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum OnboardingPromptType {
    /// Options as buttons
    MultipleChoice = 0,
    /// Options in a dropdown
    Dropdown = 1,
}

/// [Welcome Screen](https://discord.com/developers/docs/resources/guild#welcome-screen-object):
/// shown to new members of community guilds
#[derive(Debug, Clone, Deserialize)]
pub struct WelcomeScreen {
    /// The server description shown
    pub description: Option<String>,

    /// Up to 5 channels suggested to new members
    pub welcome_channels: Vec<WelcomeScreenChannel>,
}

/// [Welcome Screen Channel](https://discord.com/developers/docs/resources/guild#welcome-screen-object-welcome-screen-channel-structure);
/// also what `rest::ModifyWelcomeScreen` sets
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct WelcomeScreenChannel {
    /// The channel
    pub channel_id: Snowflake,

    /// What it is for, 1-50 characters
    pub description: String,

    /// A custom emoji shown with it
    pub emoji_id: Option<Snowflake>,

    /// A unicode emoji (or the custom emoji's name) shown with it
    pub emoji_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_onboarding_and_welcome_screens() {
        let onboarding: GuildOnboarding = serde_json::from_value(json!({
            "guild_id": "5", "enabled": true, "mode": 1, "default_channel_ids": ["6"],
            "prompts": [{
                "id": "9", "type": 1, "title": "What do you write?", "single_select": false,
                "required": true, "in_onboarding": true,
                "options": [{
                    "id": "10", "title": "Rust", "description": null, "role_ids": ["7"],
                    "channel_ids": [], "emoji": { "id": null, "name": "crab", "animated": false },
                }],
            }],
        }))
        .unwrap();
        assert_eq!(onboarding.mode, OnboardingMode::Advanced);
        let prompt = &onboarding.prompts[0];
        assert_eq!(prompt.t, OnboardingPromptType::Dropdown);
        assert_eq!(
            prompt.options[0].emoji.as_ref().unwrap().name.as_deref(),
            Some("crab")
        );

        let screen: WelcomeScreen = serde_json::from_value(json!({
            "description": "Welcome!",
            "welcome_channels": [
                { "channel_id": "8", "description": "Say hi", "emoji_id": null, "emoji_name": null },
            ],
        }))
        .unwrap();
        assert_eq!(screen.welcome_channels[0].description, "Say hi");
    }
}
