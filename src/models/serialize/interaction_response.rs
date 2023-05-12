use std::collections::HashMap;

use serde::{ser::SerializeMap, Serialize};

use crate::models::{ActionRow, AllowedMentions, Embed, MessageFlags, PartialAttachment};

const TYPE_KEY: &str = "type";
const DATA_KEY: &str = "data";

/// [Interaction Response Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-response-object-interaction-response-structure)
#[derive(Debug)]
pub enum InteractionResponse {
    /// ACK to a `ping`
    Pong,

    /// Respond to an interaction with a message
    ChannelMessageWithSource(MessageCallbackData),

    /// ACK an interaction and edit a response later, the user sees a loading state
    DeferredChannelMessageWithSource,

    /// for components, ACK an interaction and edit the original message later; the user does not see a loading state
    DeferredUpdateMessage,

    /// for components, edit the message the component was attached to
    UpdateMessage(MessageCallbackData),

    /// respond to an autocomplete interaction with suggested choices
    ApplicationCommandAutocompleteResult(AutocompleteCallbackData),

    /// respond to an interaction with a popup modal
    Modal(ModalCallbackData),
}

impl InteractionResponse {
    pub fn respond_with_message(content: String) -> Self {
        InteractionResponse::ChannelMessageWithSource(MessageCallbackData {
            tts: None,
            content: Some(content),
            embeds: None,
            allowed_mentions: None,
            flags: None,
            components: None,
            attachments: None,
        })
    }

    pub fn respond_with_embed(embed: Embed) -> Self {
        InteractionResponse::ChannelMessageWithSource(MessageCallbackData {
            tts: None,
            content: None,
            embeds: Some(vec![embed]),
            allowed_mentions: None,
            flags: None,
            components: None,
            attachments: None,
        })
    }

    pub fn respond_with_autocomplete_choices(choices: Vec<ApplicationCommandOptionChoice>) -> Self {
        InteractionResponse::ApplicationCommandAutocompleteResult(AutocompleteCallbackData {
            choices,
        })
    }
}

impl Serialize for InteractionResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        match self {
            InteractionResponse::Pong => {
                map.serialize_entry(TYPE_KEY, &1)?;
            }
            InteractionResponse::ChannelMessageWithSource(data) => {
                map.serialize_entry(TYPE_KEY, &4)?;
                map.serialize_entry(DATA_KEY, &data)?;
            }
            InteractionResponse::DeferredChannelMessageWithSource => {
                map.serialize_entry(TYPE_KEY, &5)?;
            }
            InteractionResponse::DeferredUpdateMessage => {
                map.serialize_entry(TYPE_KEY, &6)?;
            }
            InteractionResponse::UpdateMessage(data) => {
                map.serialize_entry(TYPE_KEY, &7)?;
                map.serialize_entry(DATA_KEY, &data)?;
            }
            InteractionResponse::ApplicationCommandAutocompleteResult(data) => {
                map.serialize_entry(TYPE_KEY, &8)?;
                map.serialize_entry(DATA_KEY, &data)?;
            }
            InteractionResponse::Modal(data) => {
                map.serialize_entry(TYPE_KEY, &9)?;
                map.serialize_entry(DATA_KEY, &data)?;
            }
        };
        map.end()
    }
}

/// [Message Callback Data Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-response-object-messages)
#[derive(Debug, Serialize)]
pub struct MessageCallbackData {
    /// is the response TTS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,

    /// message content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// supports up to 10 embeds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,

    /// [allowed mentions](https://discord.com/developers/docs/resources/channel#allowed-mentions-object) object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<AllowedMentions>,

    /// [message flags](https://discord.com/developers/docs/resources/channel#message-object-message-flags) combined as a [bitfield](https://en.wikipedia.org/wiki/Bit_field) (only SUPPRESS_EMBEDS and EPHEMERAL can be set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,

    /// message components
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<ActionRow>>,

    /// attachment objects with filename and description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<PartialAttachment>>,
}

#[derive(Debug, Serialize)]
pub struct AutocompleteCallbackData {
    /// autocomplete choices (max of 25 choices)
    pub choices: Vec<ApplicationCommandOptionChoice>,
}

/// [Application Command Option Choice Structure](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-option-choice-structure)
#[derive(Debug, Serialize)]
pub struct ApplicationCommandOptionChoice {
    /// 1-100 character choice name
    pub name: String,

    /// Localization dictionary for the name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// Value for the choice, up to 100 characters if string
    pub value: ApplicationCommandOptionChoiceValue,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum ApplicationCommandOptionChoiceValue {
    String(String),
    Integer(i64),
    Double(f64),
}

/// [Modal](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-response-object-modal)
#[derive(Debug, Serialize)]
pub struct ModalCallbackData {
    /// a developer-defined identifier for the modal, max 100 characters
    pub custom_id: String,

    /// the title of the popup modal, max 45 characters
    pub title: String,

    /// between 1 and 5 (inclusive) components that make up the modal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<ActionRow>>,

    /// is the response TTS
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,

    /// message content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// supports up to 10 embeds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<Embed>>,

    /// [allowed mentions](https://discord.com/developers/docs/resources/channel#allowed-mentions-object) object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<AllowedMentions>,

    /// [message flags](https://discord.com/developers/docs/resources/channel#message-object-message-flags) combined as a [bitfield](https://en.wikipedia.org/wiki/Bit_field) (only SUPPRESS_EMBEDS and EPHEMERAL can be set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<MessageFlags>,
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn serialize_test() {
        let response = InteractionResponse::ChannelMessageWithSource(MessageCallbackData {
            tts: None,
            content: Some(String::from("hello")),
            embeds: None,
            allowed_mentions: None,
            flags: None,
            components: None,
            attachments: None,
        });

        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    }
}
