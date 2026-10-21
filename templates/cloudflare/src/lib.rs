//! A Discord bot on Cloudflare Workers, built with Composure:
//! <https://bluefrog130.github.io/composure-rs/>

use composure::{app, command, command::Ctx};
use composure_adapter_cloudflare::CloudflareInteractionBot;
use worker::{Context, Env, Request, Response, event};

/// Say hello
#[command]
fn hello(ctx: Ctx) -> String {
    let name = ctx.user().map_or("stranger", |user| user.display_name());
    format!("Hello, {name}!")
}

/// Everything the bot serves; `src/bin/sync.rs` registers its Commands with Discord
#[app(handlers(hello))]
pub struct Bot;

#[event(fetch)]
async fn fetch(req: Request, env: Env, ctx: Context) -> worker::Result<Response> {
    CloudflareInteractionBot::new(req, env, ctx)
        .with_handler(Bot)
        .process()
        .await
}

#[cfg(test)]
mod tests {
    use composure::testing::{self, ResponseExt, TestPlatform};

    use super::Bot;

    #[test]
    fn hello_greets_the_user() {
        let response = testing::answer(&Bot, testing::command("hello"), &TestPlatform::new());
        assert_eq!(response.text().as_deref(), Some("Hello, Alice!"));
    }
}
