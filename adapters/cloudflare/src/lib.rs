#![doc = include_str!("../README.md")]

use async_trait::async_trait;
use composure::models::{
    ApplicationCommandInteraction, Embed, Interaction, InteractionResponse,
    MessageComponentInteraction, ModalSubmitInteraction,
};
use worker::{Env, Headers, Request, Response, console_debug, console_error, console_warn};

#[derive(Debug)]
pub enum Error {
    CommandNotFound(String),
    ValidationError,
    WorkerError(worker::Error),
    NoCommandHandler,
}

pub type Result<T> = std::result::Result<T, Error>;

const SIGNATURE_HEADER: &str = "X-Signature-Ed25519";
const TIMESTAMP_HEADER: &str = "X-Signature-Timestamp";
const PUBLIC_KEY_SECRET: &str = "DISCORD_PUBLIC_KEY";

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
pub fn validate_request(env: &Env, headers: &Headers, body: &[u8]) -> Result<()> {
    let signature = headers.get(SIGNATURE_HEADER).map_err(Error::WorkerError)?;
    let timestamp = headers.get(TIMESTAMP_HEADER).map_err(Error::WorkerError)?;

    let public_key = env
        .secret(PUBLIC_KEY_SECRET)
        .map_err(Error::WorkerError)?
        .to_string();

    verify(
        &public_key,
        signature.as_deref(),
        timestamp.as_deref(),
        body,
    )
}

fn verify(
    public_key: &str,
    signature: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
) -> Result<()> {
    let (Some(signature), Some(timestamp)) = (signature, timestamp) else {
        return Err(Error::ValidationError);
    };

    composure::auth::validate_request(public_key, signature, timestamp, body)
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
        error: worker::Error,
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
pub async fn handle_interaction<F: CloudflareCommandHandler>(
    public_key: &str,
    signature: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
    handler: Option<&F>,
) -> InteractionOutcome {
    if verify(public_key, signature, timestamp, body).is_err() {
        return InteractionOutcome::Unauthorized;
    }

    let interaction: Interaction = match serde_json::from_slice(body) {
        Ok(interaction) => interaction,
        Err(e) => return InteractionOutcome::BadRequest(e),
    };

    match dispatch(interaction, handler).await {
        Ok(response) => InteractionOutcome::Respond(response),
        Err(error) => InteractionOutcome::HandlerFailed {
            error,
            response: handler_error_response(),
        },
    }
}

/// Interaction bot for Cloudflare
pub struct CloudflareInteractionBot<F: CloudflareCommandHandler + 'static> {
    req: Request,
    env: Env,
    handler: Option<F>,
}

impl<F: CloudflareCommandHandler + 'static> CloudflareInteractionBot<F> {
    /// Creates a new Cloudflare interaction bot
    pub fn new(req: Request, env: Env) -> Self {
        Self {
            req,
            env,
            handler: None,
        }
    }

    pub fn with_handler(mut self, handler: F) -> Self {
        self.handler = Some(handler);
        self
    }

    /// Validates and handles the request. Every outcome is an HTTP response:
    /// bad signature is 401, unparseable body is 400, a missing public key secret is 500,
    /// handler errors are logged and answered with an ephemeral error message.
    pub async fn process(mut self) -> worker::Result<Response> {
        console_debug!("Processing request");

        let bytes = match self.req.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                console_error!("Failed to read body: {}", e);
                return Response::error("Bad request", 400);
            }
        };

        let public_key = match self.env.secret(PUBLIC_KEY_SECRET) {
            Ok(secret) => secret.to_string(),
            Err(e) => {
                console_error!("Missing {} secret: {}", PUBLIC_KEY_SECRET, e);
                return Response::error("Internal error", 500);
            }
        };
        let headers = self.req.headers();
        let signature = headers.get(SIGNATURE_HEADER).ok().flatten();
        let timestamp = headers.get(TIMESTAMP_HEADER).ok().flatten();

        let outcome = handle_interaction(
            &public_key,
            signature.as_deref(),
            timestamp.as_deref(),
            &bytes,
            self.handler.as_ref(),
        )
        .await;

        match outcome {
            InteractionOutcome::Unauthorized => {
                console_warn!("Validation failed");
                Response::error("Validation failed", 401)
            }
            InteractionOutcome::BadRequest(e) => {
                console_error!("Failed to parse interaction: {}", e);
                Response::error("Bad request", 400)
            }
            InteractionOutcome::Respond(response) => Response::from_json(&response),
            InteractionOutcome::HandlerFailed { error, response } => {
                console_error!("Handler error: {}", error);
                Response::from_json(&response)
            }
        }
    }
}

/// Routes an interaction to the handler. Ping is answered with Pong.
async fn dispatch<F: CloudflareCommandHandler>(
    interaction: Interaction,
    handler: Option<&F>,
) -> worker::Result<InteractionResponse> {
    match interaction {
        Interaction::Ping(_) => Ok(InteractionResponse::Pong),
        Interaction::ApplicationCommand(command) => match handler {
            Some(handler) => handler.command(command).await,
            None => Ok(InteractionResponse::respond_with_embed(
                Embed::new()
                    .with_title("No command handler")
                    .with_color(0xf04747),
            )),
        },
        Interaction::MessageComponent(component) => match handler {
            Some(handler) => handler.component(component).await,
            None => Ok(InteractionResponse::respond_with_embed(
                Embed::new()
                    .with_title("No component handler")
                    .with_color(0xf04747),
            )),
        },
        Interaction::ApplicationCommandAutocomplete(autocomplete) => match handler {
            Some(handler) => handler.autocomplete(autocomplete).await,
            None => Ok(unsupported_autocomplete_response()),
        },
        Interaction::ModalSubmit(modal) => match handler {
            Some(handler) => handler.modal_submit(modal).await,
            None => Ok(unsupported_modal_response()),
        },
    }
}

/// Sent when a handler returns an error, so the user sees a failure instead of
/// Discord's "interaction failed".
fn handler_error_response() -> InteractionResponse {
    InteractionResponse::respond_with_ephemeral_message(String::from(
        "Something went wrong handling this interaction.",
    ))
}

fn unsupported_autocomplete_response() -> InteractionResponse {
    InteractionResponse::respond_with_autocomplete_choices(Vec::new())
}

fn unsupported_modal_response() -> InteractionResponse {
    InteractionResponse::respond_with_ephemeral_message(String::from(
        "This bot does not handle modal submissions.",
    ))
}

/// Handles interactions dispatched by [`CloudflareInteractionBot`].
///
/// Workers futures are not `Send`, so implement with `#[async_trait(?Send)]`.
/// An `Err` from any method is logged and answered with an ephemeral error message.
#[async_trait(?Send)]
pub trait CloudflareCommandHandler {
    async fn command(
        &self,
        command: ApplicationCommandInteraction,
    ) -> worker::Result<InteractionResponse>;

    async fn component(
        &self,
        component: MessageComponentInteraction,
    ) -> worker::Result<InteractionResponse>;

    /// Autocomplete for command options. Defaults to no choices.
    async fn autocomplete(
        &self,
        _autocomplete: ApplicationCommandInteraction,
    ) -> worker::Result<InteractionResponse> {
        Ok(unsupported_autocomplete_response())
    }

    /// Modal submissions. Defaults to an ephemeral "not handled" message.
    async fn modal_submit(
        &self,
        _modal: ModalSubmitInteraction,
    ) -> worker::Result<InteractionResponse> {
        Ok(unsupported_modal_response())
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::{Value, json};

    use super::*;

    struct EchoHandler;

    #[async_trait(?Send)]
    impl CloudflareCommandHandler for EchoHandler {
        async fn command(
            &self,
            command: ApplicationCommandInteraction,
        ) -> worker::Result<InteractionResponse> {
            Ok(InteractionResponse::respond_with_message(format!(
                "command {}",
                command.data.name
            )))
        }

        async fn component(
            &self,
            component: MessageComponentInteraction,
        ) -> worker::Result<InteractionResponse> {
            Ok(InteractionResponse::respond_with_message(format!(
                "component {}",
                component.data.custom_id
            )))
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

    struct FailingHandler;

    #[async_trait(?Send)]
    impl CloudflareCommandHandler for FailingHandler {
        async fn command(
            &self,
            _command: ApplicationCommandInteraction,
        ) -> worker::Result<InteractionResponse> {
            Err(worker::Error::RustError("boom".into()))
        }

        async fn component(
            &self,
            _component: MessageComponentInteraction,
        ) -> worker::Result<InteractionResponse> {
            Err(worker::Error::RustError("boom".into()))
        }
    }

    fn modal_interaction() -> Interaction {
        interaction(5, Some(json!({ "custom_id": "modal", "components": [] })))
    }

    fn respond(interaction: Interaction) -> Value {
        let response = block_on(dispatch(interaction, Some(&EchoHandler))).unwrap();
        serde_json::to_value(response).unwrap()
    }

    const PUBLIC_KEY: &str = "852aec10972ef6dd0431747902c779342cc411ad6d42c2de16ef4c87895c61ad";
    const SIGNATURE: &str = "c91641b5c3d12f9c819d9b5c568ef7d660e7f9abc2c312f296c562f6d7b028dac80c6c8e5c8a11f7a21ee28dbb8c6cf2762118bee45c00b2df78065b3b59f20c";

    #[test]
    fn ping_returns_pong() {
        assert_eq!(respond(interaction(1, None)), json!({ "type": 1 }));
    }

    #[test]
    fn command_reaches_handler() {
        let response = respond(command_interaction(2));
        assert_eq!(response["type"], 4);
        assert_eq!(response["data"]["content"], "command ping");
    }

    #[test]
    fn component_reaches_handler() {
        let response = respond(component_interaction());
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
            Some(&EchoHandler),
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
        let response = respond(command_interaction(4));
        assert_eq!(response, json!({ "type": 8, "data": { "choices": [] } }));
    }

    #[test]
    fn modal_submit_defaults_to_ephemeral_message() {
        let response = respond(modal_interaction());
        assert_eq!(response["type"], 4);
        assert_eq!(response["data"]["flags"], 64);
    }

    #[test]
    fn no_handler_does_not_panic() {
        for interaction in [
            command_interaction(2),
            component_interaction(),
            command_interaction(4),
            modal_interaction(),
        ] {
            assert!(block_on(dispatch::<EchoHandler>(interaction, None)).is_ok());
        }
    }

    #[test]
    fn handler_error_becomes_ephemeral_message() {
        let result = block_on(dispatch(command_interaction(2), Some(&FailingHandler)));
        assert!(result.is_err());

        let response = serde_json::to_value(handler_error_response()).unwrap();
        assert_eq!(response["type"], 4);
        assert_eq!(response["data"]["flags"], 64);
    }
}
