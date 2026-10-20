#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
mod log;
mod state;
#[cfg(feature = "tracing")]
pub mod tracing;
mod transport;

use std::{borrow::Cow, cell::RefCell, rc::Rc};

pub use composure::adapter::{
    DEFAULT_EVENTS_PATH, EventOutcome, InteractionOutcome, handle_event, handle_event_with_key,
    handle_interaction, handle_interaction_with_key,
};
use composure::adapter::{
    FilesError, PUBLIC_KEY_VAR as PUBLIC_KEY_SECRET, SIGNATURE_HEADER, TIMESTAMP_HEADER,
    send_with_files, verify,
};
pub use composure::auth::PublicKey;
pub use composure::command::{EventHandler, HandlerError, InteractionHandler, NoHandler};
use composure::types::InteractionResponse;
use state::log_handler_error;
pub use state::{Worker, bot_client};
pub use transport::FetchTransport;
use worker::{Context, Env, Headers, Request, Response};

/// Why a request could not be validated
#[derive(Debug)]
pub enum Error {
    /// The signature is missing or invalid, or the public key is not a valid key
    ValidationError,
    /// The Workers runtime failed: a missing secret, an unreadable request, ...
    WorkerError(worker::Error),
}

/// This crate's result
pub type Result<T> = std::result::Result<T, Error>;

/// Validates a request from Discord
///
/// Missing or invalid signature headers are an [`Error::ValidationError`].
///
/// # Arguments
///
/// * `env` - The environment variables for the worker
/// * `headers` - The headers of the request
/// * `body` - The body of the request
///
pub async fn validate_request(env: &Env, headers: &Headers, body: &[u8]) -> Result<()> {
    let signature = headers.get(SIGNATURE_HEADER).map_err(Error::WorkerError)?;
    let timestamp = headers.get(TIMESTAMP_HEADER).map_err(Error::WorkerError)?;
    let public_key = public_key(env).await?;

    if verify(
        &public_key,
        signature.as_deref(),
        timestamp.as_deref(),
        body,
    )
    .await
    {
        Ok(())
    } else {
        Err(Error::ValidationError)
    }
}

thread_local! {
    /// The `DISCORD_PUBLIC_KEY` secret, parsed (and with `web-crypto`, imported) on the first
    /// request and reused for the life of the isolate. A changed secret is a new deployment, so
    /// a new isolate.
    static PUBLIC_KEY: RefCell<Option<Rc<PublicKey>>> = const { RefCell::new(None) };
}

/// The cached public key. A missing secret is an [`Error::WorkerError`]; one that is not a valid
/// key is an [`Error::ValidationError`] (and is read again next time).
async fn public_key(env: &Env) -> Result<Rc<PublicKey>> {
    if let Some(key) = PUBLIC_KEY.with_borrow(Option::clone) {
        return Ok(key);
    }
    let hex = env
        .secret(PUBLIC_KEY_SECRET)
        .map_err(Error::WorkerError)?
        .to_string();
    let key = Rc::new(
        PublicKey::from_hex(&hex)
            .await
            .map_err(|_| Error::ValidationError)?,
    );
    PUBLIC_KEY.set(Some(key.clone()));
    Ok(key)
}

/// Interaction bot for Cloudflare. The handler receives a [`Worker`] (the worker's [`Env`] and
/// [`Context`]) as its State. Webhook Events on the events path (default `/events`) go to the
/// same handler's [`EventHandler`] implementation.
pub struct CloudflareInteractionBot<H = NoHandler> {
    req: Request,
    state: Worker,
    handler: H,
    events_path: Cow<'static, str>,
}

impl CloudflareInteractionBot {
    /// Creates a new Cloudflare interaction bot. Without [`with_handler`](Self::with_handler)
    /// every interaction gets the [`InteractionHandler`] defaults.
    pub fn new(req: Request, env: Env, ctx: Context) -> Self {
        Self {
            req,
            state: Worker::new(env, ctx),
            handler: NoHandler,
            events_path: Cow::Borrowed(DEFAULT_EVENTS_PATH),
        }
    }
}

impl<H: InteractionHandler<Worker>> CloudflareInteractionBot<H> {
    /// Handles interactions with `handler`: an `#[app]`, an App `with_fallback`, or any
    /// [`InteractionHandler`]
    pub fn with_handler<T: InteractionHandler<Worker>>(
        self,
        handler: T,
    ) -> CloudflareInteractionBot<T> {
        CloudflareInteractionBot {
            req: self.req,
            state: self.state,
            handler,
            events_path: self.events_path,
        }
    }

    /// Serves Webhook Events on `path` instead of `/events`; set the app's Webhook Events URL to
    /// match. Every other path serves Interactions.
    pub fn with_events_path(mut self, path: impl Into<Cow<'static, str>>) -> Self {
        self.events_path = path.into();
        self
    }
}

impl<H: InteractionHandler<Worker> + EventHandler<Worker> + Send + 'static>
    CloudflareInteractionBot<H>
{
    /// Validates and handles the request. Every outcome is an HTTP response:
    /// bad signature is 401, unparseable body is 400, a missing public key secret is 500,
    /// handler errors are logged and answered with an ephemeral error message. Responses with
    /// files are sent through the callback endpoint and the request is answered with 202.
    /// Webhook Events are acknowledged with 204 and handled as Background Work.
    pub async fn process(mut self) -> worker::Result<Response> {
        let bytes = match self.req.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("failed to read request body", error = e);
                return Response::error("Bad request", 400);
            }
        };

        let public_key = match public_key(&self.state).await {
            Ok(key) => key,
            Err(Error::WorkerError(e)) => {
                error!("missing DISCORD_PUBLIC_KEY secret", error = e);
                return Response::error("Internal error", 500);
            }
            Err(Error::ValidationError) => {
                error!("DISCORD_PUBLIC_KEY secret is not a valid public key");
                return Response::error("Validation failed", 401);
            }
        };
        let headers = self.req.headers();
        let signature = headers.get(SIGNATURE_HEADER).ok().flatten();
        let timestamp = headers.get(TIMESTAMP_HEADER).ok().flatten();

        if self.req.path() == self.events_path {
            let outcome = handle_event_with_key(
                &public_key,
                signature.as_deref(),
                timestamp.as_deref(),
                &bytes,
                self.handler,
                &self.state,
            )
            .await;
            return match outcome {
                EventOutcome::Unauthorized => {
                    warn!("event signature validation failed");
                    Response::error("Validation failed", 401)
                }
                EventOutcome::BadRequest(e) => {
                    error!("failed to parse event", error = e);
                    Response::error("Bad request", 400)
                }
                EventOutcome::Acknowledged => {
                    let headers = Headers::new();
                    headers.set("Content-Type", "application/json")?;
                    Ok(Response::empty()?.with_status(204).with_headers(headers))
                }
            };
        }

        let outcome = handle_interaction_with_key(
            &public_key,
            signature.as_deref(),
            timestamp.as_deref(),
            &bytes,
            &self.handler,
            &self.state,
        )
        .await;

        match outcome {
            InteractionOutcome::Unauthorized => {
                warn!("interaction signature validation failed");
                Response::error("Validation failed", 401)
            }
            InteractionOutcome::BadRequest(e) => {
                error!("failed to parse interaction", error = e);
                Response::error("Bad request", 400)
            }
            InteractionOutcome::Respond(response) => reply(&bytes, &response).await,
            InteractionOutcome::HandlerFailed { error, response } => {
                log_handler_error(&error);
                reply(&bytes, &response).await
            }
        }
    }
}

/// The response as the HTTP reply, or, when it carries files (multipart, which the reply cannot
/// be), through the callback endpoint followed by an empty 202
async fn reply(body: &[u8], response: &InteractionResponse) -> worker::Result<Response> {
    if response.files().is_empty() {
        return Response::from_json(response);
    }
    match send_with_files(FetchTransport, body, response).await {
        Ok(()) => Ok(Response::empty()?.with_status(202)),
        Err(e @ FilesError::MissingCallback) => {
            error!("failed to send response with files", error = e);
            Response::error("Bad request", 400)
        }
        Err(e) => {
            error!("failed to send response with files", error = e);
            Response::error("Internal error", 500)
        }
    }
}
