//! The bot's building blocks: a container per tone, so every message looks alike.
//!
//! ```text
//! <ui::Success title="Saved">"Announcements will appear in " {channel}</ui::Success>
//! <ui::Error>"That release does not exist."</ui::Error>
//! ```
//!
//! Each sets its accent color itself instead of leaving it to the Theme, so messages sent through
//! REST (announcements, feedback), which are never themed, match the replies.

use composure::{
    command::{HandlerError, View},
    rest::{BotClient, CreateMessage, Transport},
    rsx,
    types::{Message, MessageFlags, Snowflake},
    view,
};

use crate::THEME;

/// Posts `view` to `channel` as a Components V2 message
pub async fn send<T: Transport>(
    bot: &BotClient<T>,
    channel: Snowflake,
    view: View,
) -> Result<Message, HandlerError> {
    let components = view.into_components()?;
    let message = bot
        .create_message(
            channel,
            &CreateMessage {
                components: Some(&components),
                flags: Some(MessageFlags::IsComponentsV2),
                ..Default::default()
            },
        )
        .await?
        .model()?;
    Ok(message)
}

/// Discord's grey, for neutral information
pub const SECONDARY: u32 = 0x4e5058;

/// A container with `accent` and an optional `### title` above `children`
fn panel(accent: u32, title: Option<String>, children: View) -> View {
    rsx! {
        <Container accent_color=accent>
            {title.map(|title| rsx! { <Text>"### " {title}</Text> })}
            {children}
        </Container>
    }
}

/// The brand color: pages, cards, announcements
#[view]
pub fn Primary(title: Option<String>, children: View) -> View {
    panel(THEME.primary, title, children)
}

/// Neutral information: status, notices for maintainers
#[view]
pub fn Secondary(title: Option<String>, children: View) -> View {
    panel(SECONDARY, title, children)
}

/// Something worked
#[view]
pub fn Success(title: Option<String>, children: View) -> View {
    panel(THEME.success, title, children)
}

/// Worked, with a catch
#[view]
pub fn Warning(title: Option<String>, children: View) -> View {
    panel(THEME.warning, title, children)
}

/// Something failed, said in the bot's words (handler errors are themed embeds in the same color)
#[view]
pub fn Error(title: Option<String>, children: View) -> View {
    panel(THEME.error, title, children)
}
