use std::fmt;

use crate::types::InteractionResponse;

use super::{Ephemeral, IntoResponse, Theme};

/// Why an Interaction could not be answered normally. Adapters log it and answer with
/// [`HandlerError::response`] instead.
///
/// Any [`std::error::Error`] converts with `?`; other [`fmt::Display`] values via [`HandlerError::new`].
/// [`HandlerError::user`] is for errors the invoking user should read, like "You can't ban yourself".
#[derive(Debug)]
pub struct HandlerError {
    kind: ErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The handler or Command returned an error
    Handler,

    /// An error meant for the invoking user, shown to them as is
    User,

    /// No Command in the App matches the invoked name and type
    UnknownCommand,

    /// An Option was missing, of the wrong type, or could not be resolved
    InvalidOption,
}

impl HandlerError {
    pub fn new(message: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::Handler,
            message: message.to_string(),
        }
    }

    /// An error whose message the invoking user sees (ephemerally) instead of a generic failure
    pub fn user(message: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::User,
            message: message.to_string(),
        }
    }

    pub fn unknown_command(name: &str) -> Self {
        Self {
            kind: ErrorKind::UnknownCommand,
            message: format!("unknown command `{name}`"),
        }
    }

    pub fn invalid_option(name: &str, error: impl fmt::Display) -> Self {
        Self {
            kind: ErrorKind::InvalidOption,
            message: format!("option `{name}`: {error}"),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Logs the error as `message` in the current span: DEBUG for user errors (the user already
    /// sees them), WARN for unknown commands and invalid Options (usually Commands out of sync
    /// with Discord), ERROR for handler failures
    #[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
    pub(crate) fn trace(&self, message: &'static str) {
        match self.kind {
            ErrorKind::User => {
                debug!(error = %self, error_kind = "user", "{message}");
            }
            ErrorKind::UnknownCommand => {
                warn!(error = %self, error_kind = "unknown_command", "{message}");
            }
            ErrorKind::InvalidOption => {
                warn!(error = %self, error_kind = "invalid_option", "{message}");
            }
            ErrorKind::Handler => {
                error!(error = %self, error_kind = "handler", "{message}");
            }
        }
    }

    /// The ephemeral message the invoking user sees instead of Discord's "interaction failed"
    pub fn response(&self) -> InteractionResponse {
        self.response_with(None)
    }

    /// [`HandlerError::response`], as an [`Theme::error`] embed when there is a theme. Adapters
    /// pass the handler's [`InteractionHandler::theme`](super::InteractionHandler::theme).
    pub fn response_with(&self, theme: Option<&Theme>) -> InteractionResponse {
        let message = self.user_message();
        match theme {
            Some(theme) => Ephemeral(theme.error(message)).into_response(),
            None => InteractionResponse::respond_with_ephemeral_message(message.to_string()),
        }
    }

    fn user_message(&self) -> &str {
        match self.kind {
            ErrorKind::User => &self.message,
            ErrorKind::UnknownCommand => "Unknown command.",
            ErrorKind::Handler | ErrorKind::InvalidOption => {
                "Something went wrong handling this interaction."
            }
        }
    }
}

impl fmt::Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl<E: std::error::Error> From<E> for HandlerError {
    fn from(error: E) -> Self {
        Self::new(error)
    }
}
