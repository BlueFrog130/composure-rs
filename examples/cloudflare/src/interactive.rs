//! Components and modals. Until Component Handlers have macros, they are routed by hand in
//! [`Components`], the Command Set's fallback.

use composure::{
    command,
    command::{HandlerError, InteractionHandler},
    types::{
        ActionRow, ButtonStyle, Component, InteractionResponse, MessageCallbackData,
        MessageComponentInteraction, MessageFlags, ModalCallbackData, ModalSubmitInteraction,
        TextInputStyle,
    },
};

const COUNTER: &str = "counter:";
const FEEDBACK: &str = "feedback";

/// A message with a button that counts its clicks
#[command]
pub fn counter() -> InteractionResponse {
    InteractionResponse::ChannelMessageWithSource(counter_message(0))
}

/// The count lives in the button's custom_id, so no storage is needed
fn counter_message(count: u32) -> MessageCallbackData {
    let button = Component::new_button(
        ButtonStyle::Primary,
        Some("+1".to_string()),
        None,
        Some(format!("{COUNTER}{count}")),
        None,
        None,
    );
    MessageCallbackData {
        content: Some(format!("Clicked {count} times")),
        components: Some(vec![ActionRow::new(vec![button])]),
        ..Default::default()
    }
}

/// Send feedback through a form
#[command]
pub fn feedback() -> InteractionResponse {
    let input = |id: &str, label: &str, style, required| {
        ActionRow::new(vec![Component::new_text_input(
            id.to_string(),
            style,
            label.to_string(),
            None,
            Some(1000),
            Some(required),
            None,
            None,
        )])
    };
    InteractionResponse::Modal(ModalCallbackData {
        custom_id: FEEDBACK.to_string(),
        title: "Feedback".to_string(),
        components: Some(vec![
            input("topic", "Topic", TextInputStyle::Short, true),
            input("details", "Details", TextInputStyle::Paragraph, false),
        ]),
        tts: None,
        content: None,
        embeds: None,
        allowed_mentions: None,
        flags: None,
    })
}

/// Everything that isn't a command: button clicks and modal submits
pub struct Components;

impl<S> InteractionHandler<S> for Components {
    async fn component(
        &self,
        component: MessageComponentInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        let custom_id = &component.data.custom_id;
        if let Some(count) = custom_id.strip_prefix(COUNTER) {
            let count: u32 = count.parse().map_err(HandlerError::new)?;
            // Edits the message the button is on
            return Ok(InteractionResponse::UpdateMessage(counter_message(
                count + 1,
            )));
        }
        Ok(InteractionResponse::respond_with_ephemeral_message(
            format!("Clicked {custom_id}"),
        ))
    }

    async fn modal_submit(
        &self,
        modal: ModalSubmitInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        if modal.data.custom_id != FEEDBACK {
            return Err(HandlerError::new(format!(
                "unknown modal {}",
                modal.data.custom_id
            )));
        }
        let topic = modal.data.value("topic").unwrap_or_default();
        let details = modal
            .data
            .value("details")
            .filter(|d| !d.is_empty())
            .unwrap_or("(none)");
        Ok(InteractionResponse::ChannelMessageWithSource(
            MessageCallbackData {
                content: Some(format!("Thanks! **{topic}**: {details}")),
                flags: Some(MessageFlags::Ephemeral),
                ..Default::default()
            },
        ))
    }
}
