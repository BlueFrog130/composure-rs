use std::{any::Any, fmt::Display};

use crate::types::{Container, Embed, InteractionResponse, MessageFlags};

use super::HandlerError;

/// A value a Command can answer with
pub trait IntoResponse: Sized {
    fn into_response(self) -> InteractionResponse;

    /// The Response, or why it could not be built (a [`View`](super::View) whose `custom_id`
    /// state is too long); what handlers answer with
    fn try_into_response(self) -> Result<InteractionResponse, HandlerError> {
        Ok(self.into_response())
    }
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

/// A Components V2 message of one container
impl IntoResponse for Container {
    fn into_response(self) -> InteractionResponse {
        InteractionResponse::respond_with_components(vec![self.into()])
    }
}

/// A Response only the invoking user can see: `Ephemeral("Saved")`, `Ephemeral(embed)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ephemeral<T>(pub T);

impl<T: IntoResponse> IntoResponse for Ephemeral<T> {
    fn into_response(self) -> InteractionResponse {
        self.try_into_response()
            .unwrap_or_else(|error| error.response())
    }

    fn try_into_response(self) -> Result<InteractionResponse, HandlerError> {
        let mut response = self.0.try_into_response()?;
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
        Ok(response)
    }
}

/// For components (and modals opened from one): edit the message the component is on instead of
/// sending a new one. `Update("Saved")`, `Update(InteractionResponse::...)`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Update<T>(pub T);

impl<T: IntoResponse> IntoResponse for Update<T> {
    fn into_response(self) -> InteractionResponse {
        self.try_into_response()
            .unwrap_or_else(|error| error.response())
    }

    fn try_into_response(self) -> Result<InteractionResponse, HandlerError> {
        Ok(match self.0.try_into_response()? {
            InteractionResponse::ChannelMessageWithSource(data) => {
                InteractionResponse::UpdateMessage(data)
            }
            InteractionResponse::DeferredChannelMessageWithSource(_) => {
                InteractionResponse::DeferredUpdateMessage
            }
            other => other,
        })
    }
}

/// A Response sent as is, without the App's [`Theme`](super::Theme) filling in embed
/// colors, branding and container accent colors: `Unthemed(embed)`, `Ephemeral(Unthemed(embed))`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unthemed<T>(pub T);

impl<T: IntoResponse> IntoResponse for Unthemed<T> {
    fn into_response(self) -> InteractionResponse {
        self.try_into_response()
            .unwrap_or_else(|error| error.response())
    }

    fn try_into_response(self) -> Result<InteractionResponse, HandlerError> {
        let mut response = self.0.try_into_response()?;
        if let Some(data) = response.message_data_mut() {
            data.unthemed = true;
        }
        Ok(response)
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
        self.try_into_response()
    }
}

impl<T: IntoResponse, E: Display + 'static> CommandOutput for Result<T, E> {
    fn into_result(self) -> Result<InteractionResponse, HandlerError> {
        self.map_err(into_handler_error)?.try_into_response()
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
