use interaction_bot::models::{Interaction, InteractionResponse};
use interaction_bot::{CommandMap, InteractionBot};
use worker::{Env, Error, Headers, Method, Request, Response, Result};

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
        .get("X-Signature-Ed25519")?
        .expect("Missing Ed25519");

    let timestamp = headers
        .get("X-Signature-Timestamp")?
        .expect("Missing Timestamp");

    let public_key = env.secret("DISCORD_PUBLIC_KEY")?.to_string();

    interaction_bot::auth::validate_request(&public_key, &signature, &timestamp, body)
        .map_err(|_| worker::Error::RustError(String::from("Validation failed")))
}

/// Interaction bot for Cloudflare
pub struct CloudflareInteractionBot {
    /// Commands for the bot
    commands: CommandMap,
}

impl InteractionBot for CloudflareInteractionBot {
    /// Gets the commands
    fn get_commands(&mut self) -> &mut CommandMap {
        &mut self.commands
    }
}

impl CloudflareInteractionBot {
    /// Handles a request from Discord
    pub async fn handle_request(&mut self, mut req: Request, env: Env) -> Result<Response> {
        if req.method() != Method::Post {
            return Response::error("Method not allowed", 405);
        }

        let bytes = req.bytes().await?;
        validate_request(&env, req.headers(), &bytes)?;
        let interaction: Interaction = serde_json::from_slice(&bytes)?;

        let interaction_response = match interaction {
            Interaction::Ping(_) => InteractionResponse::Pong,
            Interaction::ApplicationCommand(command) => {
                let command_name = command.data.name.as_str();
                let command_handler = self
                    .get_commands()
                    .get(command_name)
                    .ok_or(Error::RustError(String::from("Command not found")))?;

                command_handler(command)
            }
            Interaction::MessageComponent(_) => todo!(),
            Interaction::ApplicationCommandAutocomplete(_) => todo!(),
            Interaction::ModalSubmit(_) => todo!(),
        };

        Response::from_json(&interaction_response)
    }
}
