use std::collections::HashMap;

use serde::{Serialize, ser::SerializeMap};

use crate::types::{
    ActionRow, AllowedMentions, Component, Embed, File, MessageFlags, PartialAttachment, PollCreate,
};

const TYPE_KEY: &str = "type";
const DATA_KEY: &str = "data";

/// [Interaction Response Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-response-object-interaction-response-structure)
#[derive(Debug, Clone)]
pub enum InteractionResponse {
    /// ACK to a `ping`
    Pong,

    /// Respond to an interaction with a message
    ChannelMessageWithSource(MessageCallbackData),

    /// ACK an interaction and edit a response later, the user sees a loading state. The flags
    /// (only `EPHEMERAL`) decide whether the eventual response is ephemeral.
    DeferredChannelMessageWithSource(Option<MessageFlags>),

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
            content: Some(content),
            ..Default::default()
        })
    }

    /// Respond with a message only the invoking user can see
    pub fn respond_with_ephemeral_message(content: String) -> Self {
        InteractionResponse::ChannelMessageWithSource(MessageCallbackData {
            content: Some(content),
            flags: Some(MessageFlags::Ephemeral),
            ..Default::default()
        })
    }

    pub fn respond_with_embed(embed: Embed) -> Self {
        InteractionResponse::ChannelMessageWithSource(MessageCallbackData {
            embeds: Some(vec![embed]),
            ..Default::default()
        })
    }

    /// The message data of a message response (types 4 and 7)
    pub fn message_data(&self) -> Option<&MessageCallbackData> {
        match self {
            InteractionResponse::ChannelMessageWithSource(data)
            | InteractionResponse::UpdateMessage(data) => Some(data),
            _ => None,
        }
    }

    pub fn message_data_mut(&mut self) -> Option<&mut MessageCallbackData> {
        match self {
            InteractionResponse::ChannelMessageWithSource(data)
            | InteractionResponse::UpdateMessage(data) => Some(data),
            _ => None,
        }
    }

    pub fn into_message_data(self) -> Option<MessageCallbackData> {
        match self {
            InteractionResponse::ChannelMessageWithSource(data)
            | InteractionResponse::UpdateMessage(data) => Some(data),
            _ => None,
        }
    }

    /// Files to upload with a message response; none for other types
    pub fn files(&self) -> &[File] {
        self.message_data().map_or(&[], |data| &data.files)
    }

    /// Attaches a file to a message response; no-op for other types
    pub fn with_file(mut self, file: File) -> Self {
        if let Some(data) = self.message_data_mut() {
            data.files.push(file);
        }
        self
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
            InteractionResponse::DeferredChannelMessageWithSource(flags) => {
                map.serialize_entry(TYPE_KEY, &5)?;
                if let Some(flags) = flags {
                    map.serialize_entry(DATA_KEY, &DeferredData { flags })?;
                }
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

#[derive(Serialize)]
struct DeferredData<'a> {
    flags: &'a MessageFlags,
}

/// [Message Callback Data Structure](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-response-object-messages)
#[derive(Debug, Clone, Default, Serialize)]
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

    /// a poll sent with the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<PollCreate>,

    /// files to upload; sent as multipart, so the Adapter answers through the callback endpoint
    #[serde(skip)]
    pub files: Vec<File>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AutocompleteCallbackData {
    /// autocomplete choices (max of 25 choices)
    pub choices: Vec<ApplicationCommandOptionChoice>,
}

/// [Application Command Option Choice Structure](https://discord.com/developers/docs/interactions/application-commands#application-command-object-application-command-option-choice-structure)
#[derive(Debug, Clone, Serialize)]
pub struct ApplicationCommandOptionChoice {
    /// 1-100 character choice name
    pub name: String,

    /// Localization dictionary for the name field. Values follow the same restrictions as name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// Value for the choice, up to 100 characters if string
    pub value: ApplicationCommandOptionChoiceValue,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ApplicationCommandOptionChoiceValue {
    String(String),
    Integer(i64),
    Double(f64),
}

/// [Modal](https://discord.com/developers/docs/interactions/receiving-and-responding#interaction-response-object-modal)
#[derive(Debug, Clone, Serialize)]
pub struct ModalCallbackData {
    /// a developer-defined identifier for the modal, max 100 characters
    pub custom_id: String,

    /// the title of the popup modal, max 45 characters
    pub title: String,

    /// 1-5 top-level components: [`Label`](crate::types::Label)s wrapping inputs, and text
    /// displays (action rows with text inputs are deprecated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<Component>>,

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
mod tests {
    use super::*;

    #[test]
    pub fn serialize_test() {
        let response = InteractionResponse::ChannelMessageWithSource(MessageCallbackData {
            content: Some(String::from("hello")),
            ..Default::default()
        });

        println!("{}", serde_json::to_string_pretty(&response).unwrap());
    }

    #[test]
    pub fn serialize_deferred() {
        assert_eq!(
            serde_json::to_value(InteractionResponse::DeferredChannelMessageWithSource(None))
                .unwrap(),
            serde_json::json!({ "type": 5 })
        );
        assert_eq!(
            serde_json::to_value(InteractionResponse::DeferredChannelMessageWithSource(Some(
                MessageFlags::Ephemeral
            )))
            .unwrap(),
            serde_json::json!({ "type": 5, "data": { "flags": 64 } })
        );
    }

    #[test]
    pub fn serialize_ephemeral_message() {
        let response = InteractionResponse::respond_with_ephemeral_message(String::from("hello"));

        assert_eq!(
            serde_json::to_value(&response).unwrap(),
            serde_json::json!({ "type": 4, "data": { "content": "hello", "flags": 64 } })
        );
    }
}
