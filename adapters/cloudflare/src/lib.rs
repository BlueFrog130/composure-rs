use async_trait::async_trait;
use composure::models::{
    ApplicationCommandInteraction, Embed, Interaction, InteractionResponse,
    MessageComponentInteraction,
};
use worker::{console_debug, console_error, console_warn, Env, Headers, Request, Response};

#[derive(Debug)]
pub enum Error {
    CommandNotFound(String),
    ValidationError,
    WorkerError(worker::Error),
    NoCommandHandler,
}

pub type Result<T> = std::result::Result<T, Error>;

/// Validates a request from Discord
///
/// # Arguments
///
/// * `env` - The environment variables for the worker
/// * `headers` - The headers of the request
/// * `body` - The body of the request
///
pub fn validate_request(env: &Env, headers: &Headers, body: &[u8]) -> Result<()> {
    let signature = headers
        .get("X-Signature-Ed25519")
        .map_err(|e| Error::WorkerError(e))?
        .expect("Missing Ed25519");

    let timestamp = headers
        .get("X-Signature-Timestamp")
        .map_err(|e| Error::WorkerError(e))?
        .expect("Missing Timestamp");

    let public_key = env
        .secret("DISCORD_PUBLIC_KEY")
        .map_err(|e| Error::WorkerError(e))?
        .to_string();

    composure::auth::validate_request(&public_key, &signature, &timestamp, body)
        .map_err(|_| Error::ValidationError)
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

    pub async fn process(mut self) -> worker::Result<Response> {
        console_debug!("Processing request");

        let bytes = self.req.bytes().await?;
        let validation = validate_request(&self.env, self.req.headers(), &bytes);

        if let Err(err) = validation {
            match err {
                Error::ValidationError => {
                    console_warn!("Validation failed");
                    return Response::error("Validation failed", 401);
                }
                Error::WorkerError(e) => {
                    console_error!("Worker error: {}", e);
                    // passing error up
                    return Err(e);
                }
                _ => {
                    console_error!("Unknown error: {:?}", err);
                    return Response::error("Unknown error", 500);
                }
            }
        }

        // console_debug!("{}", str::from_utf8(&bytes).unwrap());

        let interaction: Interaction = serde_json::from_slice(&bytes)?;

        // console_debug!("Interaction: {:#?}", interaction);

        let interaction_response = match interaction {
            Interaction::Ping(_) => Ok(InteractionResponse::Pong),
            Interaction::ApplicationCommand(command) => match self.handler {
                Some(handler) => handler.command(command).await,
                None => Ok(InteractionResponse::respond_with_embed(
                    Embed::new()
                        .with_title("No command handler")
                        .with_color(0xf04747),
                )),
            },
            Interaction::MessageComponent(component) => match self.handler {
                Some(handler) => handler.component(component).await,
                None => Ok(InteractionResponse::respond_with_embed(
                    Embed::new()
                        .with_title("No component handler")
                        .with_color(0xf04747),
                )),
            },
            Interaction::ApplicationCommandAutocomplete(_) => todo!(),
            Interaction::ModalSubmit(_) => todo!(),
        };

        match interaction_response {
            Ok(interaction_response) => Response::from_json(&interaction_response),
            Err(e) => match e {
                _ => {
                    console_error!("Unknown error: {:?}", e);
                    Response::error("Unknown error", 400)
                }
            },
        }
    }
}

#[async_trait]
pub trait CloudflareCommandHandler {
    async fn command(
        &self,
        command: ApplicationCommandInteraction,
    ) -> worker::Result<InteractionResponse>;

    async fn component(
        &self,
        component: MessageComponentInteraction,
    ) -> worker::Result<InteractionResponse>;
}
