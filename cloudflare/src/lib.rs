use conform::models::{ApplicationCommandInteraction, Embed, Interaction, InteractionResponse};
use futures::future::BoxFuture;
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

    conform::auth::validate_request(&public_key, &signature, &timestamp, body)
        .map_err(|_| Error::ValidationError)
}

/// Interaction bot for Cloudflare
pub struct CloudflareInteractionBot<
    F: FnOnce(
        ApplicationCommandInteraction,
    ) -> BoxFuture<'static, worker::Result<InteractionResponse>>,
> {
    req: Request,
    env: Env,
    handler: Option<F>,
}

// impl InteractionBot for CloudflareInteractionBot {
//     /// Gets the commands
//     fn get_commands(&mut self) -> &mut CommandMap {
//         &mut self.commands
//     }
// }

impl<
        F: FnOnce(
            ApplicationCommandInteraction,
        ) -> BoxFuture<'static, worker::Result<InteractionResponse>>,
    > CloudflareInteractionBot<F>
{
    /// Creates a new Cloudflare interaction bot
    pub fn new(req: Request, env: Env) -> Self {
        Self {
            req,
            env,
            handler: None,
        }
    }

    pub fn with_command_handler(mut self, handler: F) -> Self {
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
                Some(handler) => handler(command).await,
                None => Ok(InteractionResponse::respond_with_embed(
                    Embed::new()
                        .with_title("No command handler".to_string())
                        .with_color(0xf04747),
                )),
            },
            Interaction::MessageComponent(_) => todo!(),
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

    // pub async fn handle_request(&mut self, mut req: Request, env: Env) -> worker::Result<Response> {
    //     console_debug!("Handling request");

    //     if req.method() != Method::Post {
    //         console_debug!("Revieved non-POST request");
    //         return Response::error("Method not allowed", 405);
    //     }

    //     let bytes = req.bytes().await?;
    //     let validation = validate_request(&env, req.headers(), &bytes);

    //     if let Err(err) = validation {
    //         match err {
    //             Error::ValidationError => {
    //                 console_warn!("Validation failed");
    //                 return Response::error("Validation failed", 401);
    //             }
    //             Error::WorkerError(e) => {
    //                 console_error!("Worker error: {}", e);
    //                 // passing error up
    //                 return Err(e);
    //             }
    //             _ => {
    //                 console_error!("Unknown error: {:?}", err);
    //                 return Response::error("Unknown error", 500);
    //             }
    //         }
    //     }

    //     // console_debug!("{}", str::from_utf8(&bytes).unwrap());

    //     let interaction: Interaction = serde_json::from_slice(&bytes)?;

    //     // console_debug!("Interaction: {:#?}", interaction);

    //     let interaction_response = match interaction {
    //         Interaction::Ping(_) => Ok(InteractionResponse::Pong),
    //         Interaction::ApplicationCommand(command) => {
    //             let command_name = command.data.name.as_str();

    //             console_debug!("Command: {}", command_name);

    //             let command_handler_result = self
    //                 .get_commands()
    //                 .get(command_name)
    //                 .ok_or(Error::CommandNotFound(command_name.to_string()));

    //             match command_handler_result {
    //                 Ok(command_handler) => Ok(command_handler(command)),
    //                 Err(e) => Err(e),
    //             }
    //         }
    //         Interaction::MessageComponent(_) => todo!(),
    //         Interaction::ApplicationCommandAutocomplete(_) => todo!(),
    //         Interaction::ModalSubmit(_) => todo!(),
    //     };

    //     match interaction_response {
    //         Ok(interaction_response) => Response::from_json(&interaction_response),
    //         Err(e) => match e {
    //             Error::CommandNotFound(command) => {
    //                 console_error!("Command not found: {}", command);
    //                 Response::error("Command not found", 404)
    //             }
    //             _ => {
    //                 console_error!("Unknown error: {:?}", e);
    //                 Response::error("Unknown error", 400)
    //             }
    //         },
    //     }
    // }
}
