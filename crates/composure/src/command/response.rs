use std::{any::Any, fmt::Display};

use crate::types::{Embed, InteractionResponse, MessageFlags};

use super::HandlerError;

/// A value a Command can answer with
pub trait IntoResponse {
    fn into_response(self) -> InteractionResponse;
}

impl IntoResponse for InteractionResponse {
    fn into_response(self) -> InteractionResponse {
        self
    }
}

impl IntoResponse for String {
    fn into_response(self) -> InteractionResponse {
        InteractionResponse::respond_with_message(self)
    }
}

impl IntoResponse for &str {
    fn into_response(self) -> InteractionResponse {
        InteractionResponse::respond_with_message(self.to_string())
    }
}

impl IntoResponse for Embed {
    fn into_response(self) -> InteractionResponse {
        InteractionResponse::respond_with_embed(self)
    }
}

/// A Response only the invoking user can see: `Ephemeral("Saved")`, `Ephemeral(embed)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ephemeral<T>(pub T);

impl<T: IntoResponse> IntoResponse for Ephemeral<T> {
    fn into_response(self) -> InteractionResponse {
        let mut response = self.0.into_response();
        match &mut response {
            InteractionResponse::ChannelMessageWithSource(data)
            | InteractionResponse::UpdateMessage(data) => {
                *data.flags.get_or_insert(MessageFlags::empty()) |= MessageFlags::Ephemeral;
            }
            InteractionResponse::DeferredChannelMessageWithSource(flags) => {
                *flags.get_or_insert(MessageFlags::empty()) |= MessageFlags::Ephemeral;
            }
            _ => {}
        }
        response
    }
}

/// For components (and modals opened from one): edit the message the component is on instead of
/// sending a new one. `Update("Saved")`, `Update(InteractionResponse::...)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Update<T>(pub T);

impl<T: IntoResponse> IntoResponse for Update<T> {
    fn into_response(self) -> InteractionResponse {
        match self.0.into_response() {
            InteractionResponse::ChannelMessageWithSource(data) => {
                InteractionResponse::UpdateMessage(data)
            }
            InteractionResponse::DeferredChannelMessageWithSource(_) => {
                InteractionResponse::DeferredUpdateMessage
            }
            other => other,
        }
    }
}

/// What a Command function may return: a Response, or a `Result` of one.
///
/// An `Err` is logged by the Adapter and shown to the user as a generic failure, except a
/// [`HandlerError::user`], whose message the user sees.
pub trait CommandOutput {
    fn into_result(self) -> Result<InteractionResponse, HandlerError>;
}

impl<T: IntoResponse> CommandOutput for T {
    fn into_result(self) -> Result<InteractionResponse, HandlerError> {
        Ok(self.into_response())
    }
}

impl<T: IntoResponse, E: Display + 'static> CommandOutput for Result<T, E> {
    fn into_result(self) -> Result<InteractionResponse, HandlerError> {
        self.map(IntoResponse::into_response)
            .map_err(into_handler_error)
    }
}

/// A [`HandlerError`] as is, keeping its kind; anything else as a generic Handler error.
/// The type check is resolved at compile time.
pub(crate) fn into_handler_error<E: Display + 'static>(error: E) -> HandlerError {
    let mut error = Some(error);
    match (&mut error as &mut dyn Any).downcast_mut::<Option<HandlerError>>() {
        Some(handler_error) => handler_error.take().expect("taken once"),
        None => HandlerError::new(error.expect("taken once")),
    }
}
