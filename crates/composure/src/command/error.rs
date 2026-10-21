use std::fmt;

use crate::types::InteractionResponse;

/// Renders a handler's error as the response the user gets instead of Discord's "interaction
/// failed": [`HandlerError::response`] unless an App sets `#[app(errors = ..)]`
///
/// ```
/// use composure::{command::HandlerError, types::{Component, Container, InteractionResponse}};
///
/// fn render(error: &HandlerError) -> InteractionResponse {
///     let message = Container::new(vec![Component::text(error.user_message())]).with_accent_color(0xf38ba8);
///     InteractionResponse::respond_with_components(vec![message.into()])
/// }
/// # let _: composure::command::ErrorRenderer = render;
/// ```
pub type ErrorRenderer = fn(&HandlerError) -> InteractionResponse;

/// Why an Interaction could not be answered normally. Adapters log it and answer with
/// [`HandlerError::response`] instead.
///
/// Any [`std::error::Error`] converts with `?`, keeping its [`source`](std::error::Error::source)
/// chain as text ([`sources`](Self::sources)); other [`fmt::Display`] values via
/// [`HandlerError::new`]. [`HandlerError::user`] is for errors the invoking user should read, like
/// "You can't ban yourself".
///
/// `{}` shows the message alone, `{:#}` the message and its causes: `request failed: connection
/// reset`.
#[derive(Debug)]
pub struct HandlerError {
    kind: ErrorKind,
    message: String,
    /// The causes of `message`, outermost first. Text rather than the errors themselves, so errors
    /// that aren't `Send + Sync + 'static` (like `worker::Error`) still convert.
    sources: Box<[String]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// What went wrong, which decides what the user sees and how it is logged
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
    /// A handler failure: logged as an error; the user sees a generic message
    pub fn new(message: impl fmt::Display) -> Self {
        Self::of_kind(ErrorKind::Handler, message.to_string())
    }

    /// An error whose message the invoking user sees (ephemerally) instead of a generic failure
    pub fn user(message: impl fmt::Display) -> Self {
        Self::of_kind(ErrorKind::User, message.to_string())
    }

    /// No Command matches `name`; usually Discord's Commands are out of sync with the App
    pub fn unknown_command(name: &str) -> Self {
        Self::of_kind(
            ErrorKind::UnknownCommand,
            format!("unknown command `{name}`"),
        )
    }

    /// The Option (or input) `name` could not be read, because of `error`
    pub fn invalid_option(name: &str, error: impl fmt::Display) -> Self {
        Self::of_kind(
            ErrorKind::InvalidOption,
            format!("option `{name}`: {error}"),
        )
    }

    fn of_kind(kind: ErrorKind, message: String) -> Self {
        Self {
            kind,
            message,
            sources: Box::default(),
        }
    }

    /// Adds `source` and its own [`source`](std::error::Error::source) chain to the causes, after
    /// any already there: `HandlerError::user("Couldn't save your note").with_source(error)`
    pub fn with_source(self, source: impl std::error::Error) -> Self {
        let mut sources = self.sources.into_vec();
        sources.push(source.to_string());
        Self {
            sources: chain(sources, source.source()),
            ..self
        }
    }

    /// What kind of error this is
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The causes of the error, outermost first: the [`source`](std::error::Error::source) chain
    /// of the error it was converted from, then those added by [`with_source`](Self::with_source)
    pub fn sources(&self) -> impl Iterator<Item = &str> {
        self.sources.iter().map(String::as_str)
    }

    /// The causes joined with `: `, as a log field; `None` (not recorded) without any
    #[cfg(feature = "tracing")]
    fn sources_field(&self) -> Option<tracing::field::DisplayValue<Sources<'_>>> {
        (!self.sources.is_empty()).then(|| tracing::field::display(Sources(&self.sources)))
    }

    /// Logs the error as `message` in the current span: DEBUG for user errors (the user already
    /// sees them), WARN for unknown commands and invalid Options (usually Commands out of sync
    /// with Discord), ERROR for handler failures
    #[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
    pub(crate) fn trace(&self, message: &'static str) {
        match self.kind {
            ErrorKind::User => {
                debug!(error = %self, sources = self.sources_field(), error_kind = "user", "{message}");
            }
            ErrorKind::UnknownCommand => {
                warn!(error = %self, sources = self.sources_field(), error_kind = "unknown_command", "{message}");
            }
            ErrorKind::InvalidOption => {
                warn!(error = %self, sources = self.sources_field(), error_kind = "invalid_option", "{message}");
            }
            ErrorKind::Handler => {
                error!(error = %self, sources = self.sources_field(), error_kind = "handler", "{message}");
            }
        }
    }

    /// The ephemeral message the invoking user sees instead of Discord's "interaction failed",
    /// with [`user_message`](Self::user_message): the [`ErrorRenderer`] Apps use by default
    pub fn response(&self) -> InteractionResponse {
        InteractionResponse::respond_with_ephemeral_message(self.user_message().to_string())
    }

    /// What the invoking user should read: the message of [`HandlerError::user`] errors, a
    /// generic one otherwise (the details are for the logs)
    pub fn user_message(&self) -> &str {
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
        f.write_str(&self.message)?;
        if f.alternate() && !self.sources.is_empty() {
            write!(f, ": {}", Sources(&self.sources))?;
        }
        Ok(())
    }
}

/// Causes joined with `: `
struct Sources<'a>(&'a [String]);

impl fmt::Display for Sources<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, source) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(": ")?;
            }
            f.write_str(source)?;
        }
        Ok(())
    }
}

/// `sources`, then `error` and its own sources as text; sized exactly, so boxing doesn't
/// reallocate (`?` on an error with causes costs a `String` per cause and one slice)
fn chain(
    mut sources: Vec<String>,
    error: Option<&(dyn std::error::Error + 'static)>,
) -> Box<[String]> {
    let causes = || std::iter::successors(error, |error| error.source());
    sources.reserve_exact(causes().count());
    sources.extend(causes().map(ToString::to_string));
    sources.into_boxed_slice()
}

impl<E: std::error::Error> From<E> for HandlerError {
    fn from(error: E) -> Self {
        Self {
            sources: chain(Vec::new(), error.source()),
            ..Self::new(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, io};

    use super::*;

    /// An error with an optional cause
    #[derive(Debug)]
    struct Failure(&'static str, Option<Box<Failure>>);

    impl fmt::Display for Failure {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.0)
        }
    }

    impl Error for Failure {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            self.1.as_deref().map(|source| source as _)
        }
    }

    fn nested() -> Failure {
        Failure(
            "request failed",
            Some(Box::new(Failure(
                "connection reset",
                Some(Box::new(Failure("os error 104", None))),
            ))),
        )
    }

    #[test]
    fn converting_keeps_the_source_chain() {
        let error = HandlerError::from(nested());
        assert_eq!(error.kind(), ErrorKind::Handler);
        assert_eq!(error.to_string(), "request failed");
        assert_eq!(
            error.sources().collect::<Vec<_>>(),
            ["connection reset", "os error 104"]
        );
        assert_eq!(
            format!("{error:#}"),
            "request failed: connection reset: os error 104"
        );
    }

    #[test]
    fn io_errors_keep_their_payloads_chain() {
        // An `io::Error` shows its payload and forwards `source` to the payload's
        let error = HandlerError::from(io::Error::other(nested()));
        assert_eq!(error.to_string(), "request failed");
        assert_eq!(
            format!("{error:#}"),
            "request failed: connection reset: os error 104"
        );
    }

    #[test]
    fn question_mark_converts() {
        fn handler() -> Result<(), HandlerError> {
            Err(nested())?
        }
        assert_eq!(handler().unwrap_err().sources().count(), 2);
    }

    #[test]
    fn with_source_wraps_by_hand() {
        let error = HandlerError::user("Couldn't save your note").with_source(nested());
        assert_eq!(error.kind(), ErrorKind::User);
        assert_eq!(error.to_string(), "Couldn't save your note");
        assert_eq!(
            format!("{error:#}"),
            "Couldn't save your note: request failed: connection reset: os error 104"
        );
        // The user still sees only the message
        let response = serde_json::to_value(error.response()).unwrap();
        assert_eq!(response["data"]["content"], "Couldn't save your note");
    }

    #[test]
    fn without_sources_alternate_is_plain() {
        let error = HandlerError::new("boom");
        assert_eq!(error.sources().count(), 0);
        assert_eq!(format!("{error:#}"), "boom");
    }
}
