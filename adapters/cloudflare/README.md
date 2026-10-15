# Cloudflare Adapter

Runs a Composure bot on Cloudflare Workers. The adapter verifies the request signature against the `DISCORD_PUBLIC_KEY` secret, answers Discord's Ping, and dispatches commands and components to your handler.

## Usage

Based on the `worker-rust` template (`npm create cloudflare@latest -- --template=cloudflare/workers-sdk/templates/experimental/worker-rust`). Workers futures are not `Send`, so implement the handler with `#[async_trait(?Send)]`.

```rust,no_run
use async_trait::async_trait;
use composure::models::{
    ApplicationCommandInteraction, Embed, InteractionResponse, MessageComponentInteraction,
};
use composure_adapter_cloudflare::{CloudflareCommandHandler, CloudflareInteractionBot};
use worker::*;

struct Bot;

#[async_trait(?Send)]
impl CloudflareCommandHandler for Bot {
    async fn command(
        &self,
        command: ApplicationCommandInteraction,
    ) -> Result<InteractionResponse> {
        match command.data.name.as_str() {
            "hello" => {
                let name = match command.common.member {
                    Some(member) => member.nick.unwrap_or(member.user.username),
                    None => "unknown user".into(),
                };
                Ok(InteractionResponse::respond_with_message(format!(
                    "Hello, {name}!"
                )))
            }
            _ => Ok(InteractionResponse::respond_with_embed(
                Embed::new()
                    .with_description("Unknown command!")
                    .with_color(0xf04747),
            )),
        }
    }

    async fn component(
        &self,
        component: MessageComponentInteraction,
    ) -> Result<InteractionResponse> {
        Ok(InteractionResponse::respond_with_message(format!(
            "Clicked {}",
            component.data.custom_id
        )))
    }
}

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    CloudflareInteractionBot::new(req, env)
        .with_handler(Bot)
        .process()
        .await
}
# fn main() {}
```

## Todo

- [ ] Make package size smaller (simple build results in ~800 kb worker size)
- [ ] Add Discord REST integration (configurable to minimize package size)
