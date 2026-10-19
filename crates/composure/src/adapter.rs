//! What every Adapter does with a request, whatever the platform: verify the signature, parse,
//! dispatch to the [`InteractionHandler`] (or hand a Webhook Event to the [`EventHandler`] as
//! Background Work), and send responses with files through the callback endpoint. Adapters map
//! the outcomes to their platform's HTTP response, so every Adapter answers alike.

use std::fmt;

use serde::Deserialize;

use crate::{
    auth::PublicKey,
    command::{EventHandler, HandlerError, InteractionHandler, Platform, dispatch, spawn_event},
    rest::{self, InteractionClient, Transport},
    types::{Interaction, InteractionResponse, Snowflake, WebhookEventPayload},
};

/// Header carrying the request's Ed25519 signature (hex)
pub const SIGNATURE_HEADER: &str = "X-Signature-Ed25519";
/// Header carrying the timestamp the signature covers
pub const TIMESTAMP_HEADER: &str = "X-Signature-Timestamp";
/// Secret / environment variable holding the application's hex public key
pub const PUBLIC_KEY_VAR: &str = "DISCORD_PUBLIC_KEY";
/// Secret / environment variable holding the bot token, for the Bot Client
pub const BOT_TOKEN_VAR: &str = "DISCORD_TOKEN";
/// Variable holding the application id, for a Bot Client outside Interactions
pub const APPLICATION_ID_VAR: &str = "DISCORD_APPLICATION_ID";
/// Path Webhook Events are served on unless the Adapter is told otherwise; every other path
/// serves Interactions
pub const DEFAULT_EVENTS_PATH: &str = "/events";

/// Whether both signature headers are present and sign `body` with `public_key`
pub async fn verify(
    public_key: &PublicKey,
    signature: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
) -> bool {
    let (Some(signature), Some(timestamp)) = (signature, timestamp) else {
        return false;
    };
    public_key.verify(signature, timestamp, body).await.is_ok()
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
    /// HTTP status to send (responses with files go through [`send_with_files`], then 202)
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

/// Verifies, parses, and dispatches one interaction request without any platform. Adapters call
/// [`handle_interaction_with_key`]; call this directly to test a handler with signed payloads.
///
/// * `public_key` - the application's hex public key
/// * `signature` / `timestamp` - the [`SIGNATURE_HEADER`] / [`TIMESTAMP_HEADER`] headers
/// * `state` - passed to the handler; the Adapter's State when running on a platform
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

/// [`handle_interaction`] with a parsed [`PublicKey`], as Adapters call it
pub async fn handle_interaction_with_key<S, H: InteractionHandler<S>>(
    public_key: &PublicKey,
    signature: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
    handler: &H,
    state: &S,
) -> InteractionOutcome {
    if !verify(public_key, signature, timestamp, body).await {
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
/// without any platform. Adapters call [`handle_event_with_key`] for the events path; call this
/// directly to test an [`EventHandler`] with signed payloads.
pub async fn handle_event<S: Platform, H: EventHandler<S> + Send + 'static>(
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

/// [`handle_event`] with a parsed [`PublicKey`], as Adapters call it
pub async fn handle_event_with_key<S: Platform, H: EventHandler<S> + Send + 'static>(
    public_key: &PublicKey,
    signature: Option<&str>,
    timestamp: Option<&str>,
    body: &[u8],
    handler: H,
    state: &S,
) -> EventOutcome {
    if !verify(public_key, signature, timestamp, body).await {
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

/// Why [`send_with_files`] could not send the response
#[derive(Debug)]
pub enum FilesError {
    /// The interaction body has no `id`, `application_id`, or `token`: answer 400
    MissingCallback,

    /// The callback request failed: answer 500
    Rest(rest::Error),
}

impl fmt::Display for FilesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCallback => {
                f.write_str("interaction without id, application_id, or token")
            }
            Self::Rest(error) => write!(f, "failed to send response with files: {error}"),
        }
    }
}

impl std::error::Error for FilesError {}

/// Sends a response carrying files (multipart, which an HTTP reply cannot be) through the
/// callback endpoint; the Adapter then answers the request with an empty 202. `body` is the
/// interaction request body: the ids and token are read from it again, so the common path keeps
/// no copy.
pub async fn send_with_files<T: Transport>(
    transport: T,
    body: &[u8],
    response: &InteractionResponse,
) -> Result<(), FilesError> {
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
    } = serde_json::from_slice(body).map_err(|_| FilesError::MissingCallback)?;
    InteractionClient::new(transport, application_id, id, token)
        .create_response(response, response.files())
        .await
        .map_err(FilesError::Rest)
}

#[cfg(test)]
mod tests {
    use crate::command::{ErrorKind, NoHandler};
    use crate::types::{ApplicationCommandInteraction, MessageComponentInteraction};
    use futures::executor::block_on;
    use serde_json::{Value, json};

    use super::*;

    struct EchoHandler;

    impl<S: Sync> InteractionHandler<S> for EchoHandler {
        async fn command(
            &self,
            command: ApplicationCommandInteraction,
            _state: &S,
        ) -> Result<InteractionResponse, HandlerError> {
            Ok(InteractionResponse::respond_with_message(format!(
                "command {}",
                command.data.name
            )))
        }

        async fn component(
            &self,
            component: MessageComponentInteraction,
            _state: &S,
        ) -> Result<InteractionResponse, HandlerError> {
            Ok(InteractionResponse::respond_with_message(format!(
                "component {}",
                component.data.custom_id
            )))
        }
    }

    struct FailingHandler;

    impl<S: Sync> InteractionHandler<S> for FailingHandler {
        async fn command(
            &self,
            _command: ApplicationCommandInteraction,
            _state: &S,
        ) -> Result<InteractionResponse, HandlerError> {
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
        assert_eq!(unknown.kind(), ErrorKind::UnknownCommand);
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
