//! The bot's building blocks: a container per tone, so every message looks alike.
//!
//! ```text
//! <ui::Success title="Saved">"Announcements will appear in " {channel}</ui::Success>
//! <ui::Error>"That release does not exist."</ui::Error>
//! ```
//!
//! Each sets its accent color from the consts below, so replies, deferred edits and messages sent
//! through REST (announcements, feedback) look alike. Handler errors use [`render_error`], the
//! App's `errors = ..`, in the same style.

use composure::{
    command::{Ephemeral, HandlerError, IntoResponse, View},
    rsx,
    types::InteractionResponse,
    view,
};

/// Composure's brand color
pub const PRIMARY: u32 = 0x7c5cff;
/// Discord's grey, for neutral information
pub const SECONDARY: u32 = 0x4e5058;
/// Discord's green
pub const SUCCESS: u32 = 0x57f287;
/// Discord's yellow
pub const WARNING: u32 = 0xfee75c;
/// Discord's red
pub const ERROR: u32 = 0xed4245;

/// Handler errors as an ephemeral [`Error`] container: the App's `errors = ..`, so the
/// framework's error replies look like the bot's own
pub fn render_error(error: &HandlerError) -> InteractionResponse {
    let message = error.user_message().to_string();
    Ephemeral(panel(ERROR, None, rsx! { <Text>{message}</Text> })).into_response()
}

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
    panel(PRIMARY, title, children)
}

/// Neutral information: status, notices for maintainers
#[view]
pub fn Secondary(title: Option<String>, children: View) -> View {
    panel(SECONDARY, title, children)
}

/// Something worked
#[view]
pub fn Success(title: Option<String>, children: View) -> View {
    panel(SUCCESS, title, children)
}

/// Worked, with a catch
#[view]
pub fn Warning(title: Option<String>, children: View) -> View {
    panel(WARNING, title, children)
}

/// Something failed, said in the bot's words; [`render_error`] answers handler errors the same way
#[view]
pub fn Error(title: Option<String>, children: View) -> View {
    panel(ERROR, title, children)
}
