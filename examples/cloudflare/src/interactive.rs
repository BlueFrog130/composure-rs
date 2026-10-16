//! Components and modals: `#[component]` / `#[modal]` handlers routed by `custom_id` prefix, with
//! their state carried in the `custom_id` itself.

use std::time::Duration;

use composure::{
    Modal, command,
    command::{
        ComponentCtx, DeferredUpdate, Ephemeral, HandlerError, Platform, Update,
        custom_id::CustomIdError,
    },
    component, modal,
    types::{
        ActionRow, ButtonStyle, Component, InteractionResponse, MessageCallbackData, SelectOption,
    },
};

/// A message with a button that counts its clicks
#[command]
pub fn counter() -> Result<InteractionResponse, CustomIdError> {
    Ok(InteractionResponse::ChannelMessageWithSource(
        counter_message(0)?,
    ))
}

/// The count lives in the button's custom_id, so no storage is needed
#[component("counter")]
pub fn counter_click(count: u32) -> Result<Update<InteractionResponse>, CustomIdError> {
    Ok(Update(InteractionResponse::ChannelMessageWithSource(
        counter_message(count + 1)?,
    )))
}

fn counter_message(count: u32) -> Result<MessageCallbackData, CustomIdError> {
    Ok(MessageCallbackData {
        content: Some(format!("Clicked {count} times")),
        components: Some(vec![ActionRow::new(vec![button(
            "+1",
            counter_click::custom_id(count)?,
        )])]),
        ..Default::default()
    })
}

/// Roll a die; re-rolls take a moment
#[command]
pub fn roll() -> Result<InteractionResponse, CustomIdError> {
    Ok(InteractionResponse::ChannelMessageWithSource(dice_message(
        "Roll the die",
        0,
    )?))
}

/// Acknowledges at once (no loading message), then edits the message from Background Work
#[component("roll")]
pub fn reroll(ctx: ComponentCtx<impl Platform>, rolls: u32) -> DeferredUpdate {
    ctx.defer_update(async move |ctx| {
        ctx.sleep(Duration::from_secs(1)).await;
        // No randomness source needed for a demo: the interaction id is unique enough
        let face = ctx.interaction().common.id.to_u64() % 6 + 1;
        let message = dice_message(&format!("Rolled a {face}"), rolls + 1)?;
        Ok::<_, HandlerError>(InteractionResponse::ChannelMessageWithSource(message))
    })
}

fn dice_message(content: &str, rolls: u32) -> Result<MessageCallbackData, CustomIdError> {
    Ok(MessageCallbackData {
        content: Some(format!("{content} ({rolls} rolls)")),
        components: Some(vec![ActionRow::new(vec![button(
            "Roll",
            reroll::custom_id(rolls)?,
        )])]),
        ..Default::default()
    })
}

fn button(label: &str, custom_id: String) -> Component {
    Component::new_button(
        ButtonStyle::Primary,
        Some(label.to_string()),
        None,
        Some(custom_id),
        None,
        None,
    )
}

/// Pick colors from a menu
#[command]
pub fn colors() -> Result<InteractionResponse, CustomIdError> {
    let options = ["Red", "Green", "Blue"]
        .map(|color| SelectOption::new(color.to_string(), color.to_lowercase(), None, None, None))
        .to_vec();
    let menu = Component::new_string_select(
        pick_colors::custom_id()?,
        Some(options),
        None,
        Some("Pick some".to_string()),
        Some(1),
        Some(3),
        None,
    );
    Ok(InteractionResponse::ChannelMessageWithSource(
        MessageCallbackData {
            content: Some("Favorite colors?".to_string()),
            components: Some(vec![ActionRow::new(vec![menu])]),
            ..Default::default()
        },
    ))
}

/// The selected values come from `ComponentCtx::values`
#[component("colors")]
pub fn pick_colors(ctx: ComponentCtx) -> Ephemeral<String> {
    Ephemeral(format!("You picked {}", ctx.values().join(", ")))
}

#[derive(Modal)]
#[modal(title = "Feedback")]
pub struct FeedbackForm {
    #[input(max_len = 100, placeholder = "What is it about?")]
    topic: String,
    #[input(paragraph, max_len = 1000)]
    details: Option<String>,
}

/// Send feedback through a form
#[command]
pub fn feedback() -> Result<InteractionResponse, CustomIdError> {
    on_feedback::show()
}

/// The submitted form arrives typed
#[modal("feedback")]
pub fn on_feedback(form: FeedbackForm) -> Ephemeral<String> {
    let details = form.details.as_deref().unwrap_or("(none)");
    Ephemeral(format!("Thanks! **{}**: {details}", form.topic))
}
