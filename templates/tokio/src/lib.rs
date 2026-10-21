//! A Discord bot on a server with tokio, built with Composure:
//! <https://bluefrog130.github.io/composure-rs/>

use composure::{app, command, command::Ctx};

/// Say hello
#[command]
fn hello(ctx: Ctx) -> String {
    let name = ctx.user().map_or("stranger", |user| user.display_name());
    format!("Hello, {name}!")
}

/// Everything the bot serves; `src/bin/sync.rs` registers its Commands with Discord
#[app(handlers(hello))]
pub struct Bot;

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
