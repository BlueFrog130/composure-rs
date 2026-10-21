//! The bot's look: its colors, a branded embed, and how handler errors are answered. A bot owns
//! its colors as plain `const`s and sets them where it builds a message, so replies, deferred
//! edits and REST messages look alike (ADR 0015).

use composure::{
    command::{Ephemeral, HandlerError, IntoResponse},
    types::{Embed, EmbedFooter, InteractionResponse},
};

/// Accents and plain embeds
pub const PRIMARY: u32 = 0xf5c2e7;
/// Something worked
pub const SUCCESS: u32 = 0x57f287;
/// Something to double-check
pub const WARNING: u32 = 0xfee75c;
/// Something failed
pub const ERROR: u32 = 0xed4245;

/// An embed in `color`, with the bot's footer
pub fn embed(color: u32) -> Embed {
    Embed::new().with_color(color).with_footer(EmbedFooter::new(
        "composure example".into(),
        None,
        None,
    ))
}

/// Handler errors as an ephemeral embed in [`ERROR`]: the App's `errors = ..`
pub fn render_error(error: &HandlerError) -> InteractionResponse {
    Ephemeral(embed(ERROR).with_description(error.user_message())).into_response()
}
