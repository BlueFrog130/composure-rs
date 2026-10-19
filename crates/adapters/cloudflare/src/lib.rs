#![doc = include_str!("../README.md")]

#[macro_use]
mod log;
mod state;
#[cfg(feature = "tracing")]
pub mod tracing;
mod transport;

use std::{borrow::Cow, cell::RefCell, rc::Rc};

pub use composure::auth::PublicKey;
pub use composure::command::{EventHandler, HandlerError, InteractionHandler, NoHandler};
use composure::{
    command::{Platform, dispatch, spawn_event},
    rest::InteractionClient,
    types::{Interaction, InteractionResponse, Snowflake, WebhookEventPayload},
};
use serde::Deserialize;
use state::log_handler_error;
pub use state::{Worker, bot_client};
pub use transport::FetchTransport;
use worker::{Context, Env, Headers, Request, Response};

#[derive(Debug)]
pub enum Error {
    ValidationError,
    WorkerError(worker::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

const SIGNATURE_HEADER: &str = "X-Signature-Ed25519";
const TIMESTAMP_HEADER: &str = "X-Signature-Timestamp";
const PUBLIC_KEY_SECRET: &str = "DISCORD_PUBLIC_KEY";

/// Path Webhook Events are served on unless changed with
/// [`CloudflareInteractionBot::with_events_path`]
pub const DEFAULT_EVENTS_PATH: &str = "/events";

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

    verify(
        &public_key,
        signature.as_deref(),
        timestamp.as_deref(),
        body,
    )
    .await
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

async fn verify(
    public_key: &PublicKey,
    signature: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
) -> Result<()> {
    let (Some(signature), Some(timestamp)) = (signature, timestamp) else {
        return Err(Error::ValidationError);
    };

    public_key
        .verify(signature, timestamp, body)
        .await
        .map_err(|_| Error::ValidationError)
}

/// What to answer one interaction request with. Produced by [`handle_interaction`].
#[derive(Debug)]
pub enum InteractionOutcome {
    /// Signature headers missing or invalid
    Unauthorized,

    /// Signature valid but the body is not an interaction
    BadRequest(serde_json::Error),

    /// The interaction was handled
    Respond(InteractionResponse),

    /// The handler returned an error; `response` is the ephemeral error message sent instead
    HandlerFailed {
        error: HandlerError,
        response: InteractionResponse,
    },
}

impl InteractionOutcome {
    /// HTTP status to send
    pub fn status(&self) -> u16 {
        match self {
            Self::Unauthorized => 401,
            Self::BadRequest(_) => 400,
            Self::Respond(_) | Self::HandlerFailed { .. } => 200,
        }
    }

    /// JSON body to send, for 200 outcomes
    pub fn response(&self) -> Option<&InteractionResponse> {
        match self {
            Self::Respond(response) | Self::HandlerFailed { response, .. } => Some(response),
            Self::Unauthorized | Self::BadRequest(_) => None,
        }
    }
}

/// Verifies, parses, and dispatches one interaction request without needing the Workers runtime.
/// [`CloudflareInteractionBot::process`] is a thin wrapper over this; call it directly to test a
/// handler with signed payloads.
///
/// * `public_key` - the application's hex public key
/// * `signature` / `timestamp` - the `X-Signature-Ed25519` / `X-Signature-Timestamp` headers
/// * `state` - passed to the handler; `worker::Env` when running on Workers
pub async fn handle_interaction<S, H: InteractionHandler<S>>(
    public_key: &str,
    signature: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
    handler: &H,
    state: &S,
) -> InteractionOutcome {
    let Ok(public_key) = PublicKey::from_hex(public_key).await else {
        return InteractionOutcome::Unauthorized;
    };
    handle_interaction_with_key(&public_key, signature, timestamp, body, handler, state).await
}

/// [`handle_interaction`] with a parsed [`PublicKey`], as [`CloudflareInteractionBot::process`]
/// calls it
pub async fn handle_interaction_with_key<S, H: InteractionHandler<S>>(
    public_key: &PublicKey,
    signature: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
    handler: &H,
    state: &S,
) -> InteractionOutcome {
    if verify(public_key, signature, timestamp, body)
        .await
        .is_err()
    {
        return InteractionOutcome::Unauthorized;
    }

    let interaction: Interaction = match serde_json::from_slice(body) {
        Ok(interaction) => interaction,
        Err(e) => return InteractionOutcome::BadRequest(e),
    };

    match dispatch(interaction, handler, state).await {
        Ok(response) => InteractionOutcome::Respond(response),
        Err(error) => InteractionOutcome::HandlerFailed {
            response: error.response_with(handler.theme()),
            error,
        },
    }
}

/// What to answer one Webhook Events request with. Produced by [`handle_event`].
#[derive(Debug)]
pub enum EventOutcome {
    /// Signature headers missing or invalid
    Unauthorized,

    /// Signature valid but the body is not a Webhook Event payload
    BadRequest(serde_json::Error),

    /// A Ping, or an event handed to the handler as Background Work
    Acknowledged,
}

impl EventOutcome {
    /// HTTP status to send; acknowledgements have no body
    pub fn status(&self) -> u16 {
        match self {
            Self::Unauthorized => 401,
            Self::BadRequest(_) => 400,
            Self::Acknowledged => 204,
        }
    }
}

/// Verifies and parses one Webhook Events request and spawns its handling as Background Work,
/// without needing the Workers runtime. [`CloudflareInteractionBot::process`] calls it for the
/// events path; call it directly to test an [`EventHandler`] with signed payloads.
pub async fn handle_event<S: Platform, H: EventHandler<S> + 'static>(
    public_key: &str,
    signature: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
    handler: H,
    state: &S,
) -> EventOutcome {
    let Ok(public_key) = PublicKey::from_hex(public_key).await else {
        return EventOutcome::Unauthorized;
    };
    handle_event_with_key(&public_key, signature, timestamp, body, handler, state).await
}

/// [`handle_event`] with a parsed [`PublicKey`], as [`CloudflareInteractionBot::process`] calls it
pub async fn handle_event_with_key<S: Platform, H: EventHandler<S> + 'static>(
    public_key: &PublicKey,
    signature: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
    handler: H,
    state: &S,
) -> EventOutcome {
    if verify(public_key, signature, timestamp, body)
        .await
        .is_err()
    {
        return EventOutcome::Unauthorized;
    }
    let payload: WebhookEventPayload = match serde_json::from_slice(body) {
        Ok(payload) => payload,
        Err(e) => return EventOutcome::BadRequest(e),
    };
    if let Some(event) = payload.event {
        spawn_event(handler, state, payload.application_id, event);
    }
    EventOutcome::Acknowledged
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
    /// Handles interactions with `handler`: a Command Set, a Command Set `with_fallback`, or any
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

impl<H: InteractionHandler<Worker> + EventHandler<Worker> + 'static> CloudflareInteractionBot<H> {
    /// Validates and handles the request. Every outcome is an HTTP response:
    /// bad signature is 401, unparseable body is 400, a missing public key secret is 500,
    /// handler errors are logged and answered with an ephemeral error message. Responses with
    /// files are sent through the callback endpoint and the request is answered with 202.
    /// Webhook Events are acknowledged with 204 and handled as Background Work.
    pub async fn process(mut self) -> worker::Result<Response> {
        let bytes = match self.req.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("failed to read body: {e}");
                return Response::error("Bad request", 400);
            }
        };

        let public_key = match public_key(&self.state).await {
            Ok(key) => key,
            Err(Error::WorkerError(e)) => {
                error!("missing {PUBLIC_KEY_SECRET} secret: {e}");
                return Response::error("Internal error", 500);
            }
            Err(Error::ValidationError) => {
                error!("{PUBLIC_KEY_SECRET} is not a valid public key");
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
                    error!("failed to parse event: {e}");
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
                error!("failed to parse interaction: {e}");
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
    let Some(client) = callback_client(body) else {
        error!("interaction without id, application_id, or token");
        return Response::error("Bad request", 400);
    };
    match client.create_response(response, response.files()).await {
        Ok(()) => Ok(Response::empty()?.with_status(202)),
        Err(e) => {
            error!("failed to send response with files: {e}");
            Response::error("Internal error", 500)
        }
    }
}

/// Reads just the ids and token again: only responses with files need them, so the common path
/// keeps no copy
fn callback_client(body: &[u8]) -> Option<InteractionClient<FetchTransport>> {
    #[derive(Deserialize)]
    struct Callback {
        id: Snowflake,
        application_id: Snowflake,
        token: String,
    }
    let Callback {
        id,
        application_id,
        token,
    } = serde_json::from_slice(body).ok()?;
    Some(InteractionClient::new(
        FetchTransport,
        application_id,
        id,
        token,
    ))
}

#[cfg(test)]
mod tests {
    use composure::types::{ApplicationCommandInteraction, MessageComponentInteraction};
    use futures::executor::block_on;
    use serde_json::{Value, json};

    use super::*;

    struct EchoHandler;

    impl<S> InteractionHandler<S> for EchoHandler {
        async fn command(
            &self,
            command: ApplicationCommandInteraction,
            _state: &S,
        ) -> std::result::Result<InteractionResponse, HandlerError> {
            Ok(InteractionResponse::respond_with_message(format!(
                "command {}",
                command.data.name
            )))
        }

        async fn component(
            &self,
            component: MessageComponentInteraction,
            _state: &S,
        ) -> std::result::Result<InteractionResponse, HandlerError> {
            Ok(InteractionResponse::respond_with_message(format!(
                "component {}",
                component.data.custom_id
            )))
        }
    }

    struct FailingHandler;

    impl<S> InteractionHandler<S> for FailingHandler {
        async fn command(
            &self,
            _command: ApplicationCommandInteraction,
            _state: &S,
        ) -> std::result::Result<InteractionResponse, HandlerError> {
            Err(HandlerError::new("boom"))
        }
    }

    fn interaction(t: u8, data: Option<Value>) -> Interaction {
        let mut value = json!({
            "id": "1100173248714518568",
            "application_id": "1052322265397739523",
            "type": t,
            "token": "token",
            "version": 1,
        });
        if let Some(data) = data {
            value["data"] = data;
        }
        serde_json::from_value(value).unwrap()
    }

    fn command_interaction(t: u8) -> Interaction {
        interaction(
            t,
            Some(json!({ "id": "1052358444704862218", "name": "ping", "type": 1 })),
        )
    }

    fn component_interaction() -> Interaction {
        interaction(
            3,
            Some(json!({ "custom_id": "button", "component_type": 2 })),
        )
    }

    fn modal_interaction() -> Interaction {
        interaction(5, Some(json!({ "custom_id": "modal", "components": [] })))
    }

    fn respond(handler: &impl InteractionHandler<()>, interaction: Interaction) -> Value {
        let response = block_on(dispatch(interaction, handler, &())).unwrap();
        serde_json::to_value(response).unwrap()
    }

    const PUBLIC_KEY: &str = "852aec10972ef6dd0431747902c779342cc411ad6d42c2de16ef4c87895c61ad";
    const SIGNATURE: &str = "c91641b5c3d12f9c819d9b5c568ef7d660e7f9abc2c312f296c562f6d7b028dac80c6c8e5c8a11f7a21ee28dbb8c6cf2762118bee45c00b2df78065b3b59f20c";

    #[test]
    fn ping_returns_pong() {
        assert_eq!(
            respond(&EchoHandler, interaction(1, None)),
            json!({ "type": 1 })
        );
    }

    #[test]
    fn command_reaches_handler() {
        let response = respond(&EchoHandler, command_interaction(2));
        assert_eq!(response["type"], 4);
        assert_eq!(response["data"]["content"], "command ping");
    }

    #[test]
    fn component_reaches_handler() {
        let response = respond(&EchoHandler, component_interaction());
        assert_eq!(response["type"], 4);
        assert_eq!(response["data"]["content"], "component button");
    }

    fn outcome(
        signature: Option<&str>,
        timestamp: Option<&str>,
        body: &[u8],
    ) -> InteractionOutcome {
        block_on(handle_interaction(
            PUBLIC_KEY,
            signature,
            timestamp,
            body,
            &EchoHandler,
            &(),
        ))
    }

    #[test]
    fn missing_signature_header_is_401() {
        assert_eq!(outcome(None, Some("1682372142"), b"{}").status(), 401);
    }

    #[test]
    fn missing_timestamp_header_is_401() {
        assert_eq!(outcome(Some(SIGNATURE), None, b"{}").status(), 401);
    }

    #[test]
    fn invalid_signature_is_401() {
        assert_eq!(
            outcome(Some(SIGNATURE), Some("1682372142"), b"{}").status(),
            401
        );
        assert_eq!(
            outcome(Some("not hex"), Some("1682372142"), b"{}").status(),
            401
        );
    }

    #[test]
    fn autocomplete_defaults_to_no_choices() {
        let response = respond(&EchoHandler, command_interaction(4));
        assert_eq!(response, json!({ "type": 8, "data": { "choices": [] } }));
    }

    #[test]
    fn modal_submit_defaults_to_ephemeral_message() {
        let response = respond(&EchoHandler, modal_interaction());
        assert_eq!(response["type"], 4);
        assert_eq!(response["data"]["flags"], 64);
    }

    #[test]
    fn no_handler_does_not_panic() {
        for interaction in [
            component_interaction(),
            command_interaction(4),
            modal_interaction(),
        ] {
            assert!(block_on(dispatch(interaction, &NoHandler, &())).is_ok());
        }
        let unknown = block_on(dispatch(command_interaction(2), &NoHandler, &())).unwrap_err();
        assert_eq!(
            unknown.kind(),
            composure::command::ErrorKind::UnknownCommand
        );
    }

    #[test]
    fn handler_error_becomes_ephemeral_message() {
        let error = block_on(dispatch(command_interaction(2), &FailingHandler, &())).unwrap_err();
        assert_eq!(error.to_string(), "boom");

        let response = serde_json::to_value(error.response()).unwrap();
        assert_eq!(response["type"], 4);
        assert_eq!(response["data"]["flags"], 64);
    }
}
