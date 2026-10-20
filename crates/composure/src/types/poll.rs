use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::{PartialEmoji, Snowflake, User};

/// [Poll](https://discord.com/developers/docs/resources/poll#poll-object) on a message
#[derive(Debug, Clone, Deserialize)]
pub struct Poll {
    /// The question; text only, max 300 characters
    pub question: PollMedia,
    /// The answers, each with its `answer_id`
    pub answers: Vec<PollAnswer>,
    /// When it ends (ISO 8601); `None` for non-expiring polls
    pub expiry: Option<String>,
    /// Whether a user can pick several answers
    pub allow_multiselect: bool,
    /// How it is laid out
    pub layout_type: PollLayoutType,
    /// Vote counts; may be missing while Discord is counting
    pub results: Option<PollResults>,
}

/// Text and/or emoji of a question or answer
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PollMedia {
    /// The text; max 300 characters for questions, 55 for answers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// An emoji, for answers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<PartialEmoji>,
}

impl PollMedia {
    /// Text without an emoji
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            emoji: None,
        }
    }
}

/// One answer of a [`Poll`]
#[derive(Debug, Clone, Deserialize)]
pub struct PollAnswer {
    /// Identifies the answer in results and votes
    pub answer_id: u32,
    /// Its text and emoji
    pub poll_media: PollMedia,
}

/// [Layout Type](https://discord.com/developers/docs/resources/poll#layout-type) of a poll
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum PollLayoutType {
    /// The only layout so far
    Default = 1,
}

/// [Poll Results](https://discord.com/developers/docs/resources/poll#poll-results-object)
#[derive(Debug, Clone, Deserialize)]
pub struct PollResults {
    /// Final once the poll ended and votes were counted
    pub is_finalized: bool,
    /// Vote counts of the answers that have votes
    pub answer_counts: Vec<PollAnswerCount>,
}

/// Votes for one answer
#[derive(Debug, Clone, Deserialize)]
pub struct PollAnswerCount {
    /// The answer's `answer_id`
    pub id: u32,
    /// How many users voted for it
    pub count: u32,
    /// Whether the bot voted for it
    pub me_voted: bool,
}

/// [Poll Create Request](https://discord.com/developers/docs/resources/poll#poll-create-request-object):
/// send with a message
#[derive(Debug, Clone, Default, Serialize)]
pub struct PollCreate {
    /// The question; text only, max 300 characters
    pub question: PollMedia,
    /// 1-10 answers
    pub answers: Vec<PollAnswerCreate>,
    /// Hours open (1-768, default 24)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    /// Whether a user can pick several answers (default false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_multiselect: Option<bool>,
    /// How it is laid out (default [`PollLayoutType::Default`])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_type: Option<PollLayoutType>,
}

impl PollCreate {
    /// A text question with text answers
    pub fn new<'a>(question: &str, answers: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            question: PollMedia::text(question),
            answers: answers
                .into_iter()
                .map(|answer| PollAnswerCreate {
                    poll_media: PollMedia::text(answer),
                })
                .collect(),
            ..Default::default()
        }
    }
}

/// One answer of a [`PollCreate`]
#[derive(Debug, Clone, Default, Serialize)]
pub struct PollAnswerCreate {
    /// Its text (max 55 characters) and optional emoji
    pub poll_media: PollMedia,
}

/// Users who voted for one answer
#[derive(Debug, Clone, Deserialize)]
pub struct PollAnswerVoters {
    /// The voters
    pub users: Vec<User>,
}

/// [Subscription](https://discord.com/developers/docs/resources/subscription#subscription-object)
/// to a premium SKU
#[derive(Debug, Clone, Deserialize)]
pub struct Subscription {
    /// The subscription's id
    pub id: Snowflake,
    /// The subscribing user
    pub user_id: Snowflake,
    /// The SKUs subscribed to
    pub sku_ids: Vec<Snowflake>,
    /// The entitlements granted by it
    pub entitlement_ids: Vec<Snowflake>,
    /// SKUs it renews into next period
    pub renewal_sku_ids: Option<Vec<Snowflake>>,
    /// ISO 8601
    pub current_period_start: String,
    /// ISO 8601
    pub current_period_end: String,
    /// 0 active, 1 ending, 2 inactive
    pub status: u8,
    /// When it was canceled (ISO 8601)
    pub canceled_at: Option<String>,
    /// ISO 3166-1 alpha-2, for the app's own subscriptions
    pub country: Option<String>,
}

/// [Soundboard Sound](https://discord.com/developers/docs/resources/soundboard#soundboard-sound-object)
#[derive(Debug, Clone, Deserialize)]
pub struct SoundboardSound {
    /// The sound's name
    pub name: String,
    /// The sound's id
    pub sound_id: Snowflake,
    /// 0-1
    pub volume: f64,
    /// The custom emoji shown with it
    pub emoji_id: Option<Snowflake>,
    /// The unicode emoji shown with it
    pub emoji_name: Option<String>,
    /// `None` for default sounds
    pub guild_id: Option<Snowflake>,
    /// Whether it can be played (servers may lose sounds when losing boosts)
    pub available: bool,
    /// Who uploaded it
    pub user: Option<User>,
}

/// [Guild Application Command Permissions](https://discord.com/developers/docs/interactions/application-commands#application-command-permissions-object-guild-application-command-permissions-structure)
#[derive(Debug, Clone, Deserialize)]
pub struct GuildCommandPermissions {
    /// The command, or the application id for permissions on every command
    pub id: Snowflake,
    /// The application
    pub application_id: Snowflake,
    /// The guild
    pub guild_id: Snowflake,
    /// The permission overwrites, up to 100
    pub permissions: Vec<CommandPermission>,
}

/// One permission overwrite of a Command: allow or deny a role, user, or channel
#[derive(Debug, Clone, Deserialize)]
pub struct CommandPermission {
    /// Role, user, or channel id (`guild_id` for @everyone, `guild_id - 1` for all channels)
    pub id: Snowflake,
    /// 1 role, 2 user, 3 channel
    #[serde(rename = "type")]
    pub kind: u8,
    /// Allowed or denied
    pub permission: bool,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn polls_round_trip() {
        let create = PollCreate::new("Lunch?", ["Pizza", "Tacos"]);
        assert_eq!(
            serde_json::to_value(&create).unwrap(),
            json!({
                "question": { "text": "Lunch?" },
                "answers": [{ "poll_media": { "text": "Pizza" } }, { "poll_media": { "text": "Tacos" } }],
            })
        );
        let poll: Poll = serde_json::from_value(json!({
            "question": { "text": "Lunch?" },
            "answers": [{ "answer_id": 1, "poll_media": { "text": "Pizza" } }],
            "expiry": "2030-01-01T00:00:00+00:00", "allow_multiselect": false, "layout_type": 1,
            "results": { "is_finalized": false, "answer_counts": [{ "id": 1, "count": 3, "me_voted": false }] },
        }))
        .unwrap();
        assert_eq!(poll.results.unwrap().answer_counts[0].count, 3);
    }
}
