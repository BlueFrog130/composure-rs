use std::fmt::Display;

use crate::models::{Embed, InteractionResponse};

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

/// What a Command function may return: a Response, or a `Result` of one whose error is shown to
/// the user as a generic failure and logged by the Adapter.
pub trait CommandOutput {
    fn into_result(self) -> Result<InteractionResponse, HandlerError>;
}

impl<T: IntoResponse> CommandOutput for T {
    fn into_result(self) -> Result<InteractionResponse, HandlerError> {
        Ok(self.into_response())
    }
}

impl<T: IntoResponse, E: Display> CommandOutput for Result<T, E> {
    fn into_result(self) -> Result<InteractionResponse, HandlerError> {
        self.map(IntoResponse::into_response)
            .map_err(HandlerError::new)
    }
}
