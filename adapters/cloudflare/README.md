# Cloudflare Adapter

This adapter is for use with Cloudflare Workers. It is a simple adapter that parses the request body and passes it to the framework.

## Usage

This sample is based off of `npm init cloudflare project_name worker-rust` template.

```rust
use composure_cloudflare::CloudflareInteractionBot;

use worker::*;

mod utils;

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    let command_handler = |interaction: ApplicationCommandInteraction| async move {
        match interaction.data.name.as_str() {
            "test" => test_handler(interaction),
            "subcommand" => {
                match interaction.data.first_option().ok_or_else(|| {
                    Error::RustError("expected subcommand or subcommand group".into())
                })? {
                    ApplicationCommandInteractionDataOption::Subcommand(command) => {
                        match command.name.as_str() {
                            "test" => test_handler(interaction),
                            _ => unknown_handler(interaction),
                        }
                    }
                    ApplicationCommandInteractionDataOption::SubcommandGroup(command) => {
                        match command.name.as_str() {
                            "group" => match command.subcommand.name.as_str() {
                                "test" => test_handler(&command, &interaction),
                                _ => unknown_handler(interaction),
                            },
                            _ => unknown_handler(interaction),
                        }
                    }
                    _ => Err(Error::RustError("expected subcommand group".into())),
                }
            }
            _ => unknown_handler(interaction),
        }
    };

    // The adapter will handle the request and return a response.
    CloudflareInteractionBot::new(req, env)
        .with_command_handler(|interaction| Box::pin(command_handler(interaction)))
        .process()
        .await
}

fn test_handler(command: ApplicationCommandInteraction) -> Result<InteractionResponse> {
    let username = match command.common.member {
        Some(member) => match member.nick {
            Some(nick) => nick,
            None => member.user.username,
        },
        None => "unknown user".into(),
    };
    let response = InteractionResponse::ChannelMessageWithSource(MessageCallbackData {
        content: Some(format!("Hello, {}!", username)),
        allowed_mentions: None,
        embeds: None,
        flags: None,
        tts: None,
        attachments: None,
        components: None,
    });

    Ok(response)
}

fn unknown_handler(_command: ApplicationCommandInteraction) -> Result<InteractionResponse> {
    Ok(InteractionResponse::respond_with_embed(
        Embed::new()
            .with_description("Unknown command!".into())
            .with_color(0xf04747),
    ))
}

```

## Todo

- [ ] Make package size smaller (simple build results in ~800 kb worker size)
- [ ] Add Discord REST integration (configurable to minimize pakacge size)
