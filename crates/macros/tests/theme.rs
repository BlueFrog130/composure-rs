//! An App's Theme: helpers through `ctx.theme()`, auto-filled responses, themed errors.

use composure::{
    Ctx, app,
    command::{
        ComponentCtx, Ephemeral, HandlerError, InteractionHandler, Theme, Unthemed, dispatch,
    },
    component, group,
    types::{Embed, Interaction},
};
use futures::executor::block_on;
use serde_json::{Value, json};

const THEME: Theme = Theme::new()
    .with_primary(0xf5c2e7)
    .with_author("Bot", Some("https://example.com/icon.png"))
    .with_footer("Footer", None);

/// Plain embed, filled in by the theme
#[composure::command]
fn plain() -> Embed {
    Embed::new().with_title("plain")
}

/// Semantic helper through the Ctx
#[composure::command]
fn saved(ctx: Ctx) -> Embed {
    ctx.theme().success("Saved")
}

/// Kept as is
#[composure::command]
fn raw() -> Ephemeral<Unthemed<Embed>> {
    Ephemeral(Unthemed(Embed::new().with_title("raw")))
}

/// Text is never turned into an embed
#[composure::command]
fn text() -> &'static str {
    "hi"
}

/// Shown as a themed error
#[composure::command]
fn refuse() -> Result<String, HandlerError> {
    Err(HandlerError::user("Not allowed"))
}

/// Subcommand reading the theme through its Group
#[composure::command]
fn info(ctx: Ctx) -> Embed {
    ctx.theme().info("nested")
}

/// Tools
#[group(subcommands(info))]
struct Tools;

#[component("warn")]
fn warn(ctx: ComponentCtx) -> Embed {
    ctx.theme().warning("careful")
}

#[app(handlers(plain, saved, raw, text, refuse, Tools, warn), theme = THEME)]
pub struct Themed;

mod unthemed {
    use super::*;

    #[app(handlers(plain, saved, refuse))]
    pub struct Unthemed;
}

mod inline {
    use super::*;

    #[app(handlers(plain), theme = Theme::new().with_primary(0x123456))]
    pub struct Inline;
}

fn interaction(t: u8, data: Value) -> Interaction {
    serde_json::from_value(json!({
        "id": "1", "application_id": "2", "type": t, "token": "t", "version": 1,
        "user": { "id": "3", "username": "ana", "discriminator": "0", "global_name": null, "avatar": null },
        "data": data,
    }))
    .unwrap()
}

fn chat(name: &str) -> Interaction {
    interaction(2, json!({ "id": "1", "name": name, "type": 1 }))
}

fn respond<H: InteractionHandler<()>>(handler: &H, interaction: Interaction) -> Value {
    let response = match block_on(dispatch(interaction, handler, &())) {
        Ok(response) => response,
        Err(error) => error.response_with(handler.theme()),
    };
    serde_json::to_value(response).unwrap()
}

fn embed(response: &Value) -> &Value {
    &response["data"]["embeds"][0]
}

#[test]
fn responses_are_filled_in() {
    let response = respond(&Themed, chat("plain"));
    let embed = embed(&response);
    assert_eq!(embed["title"], "plain");
    assert_eq!(embed["color"], 0xf5c2e7);
    assert_eq!(embed["author"]["name"], "Bot");
    assert_eq!(embed["author"]["icon_url"], "https://example.com/icon.png");
    assert_eq!(embed["footer"]["text"], "Footer");
}

#[test]
fn helpers_use_semantic_colors() {
    let response = respond(&Themed, chat("saved"));
    assert_eq!(embed(&response)["color"], Theme::DEFAULT.success);
    assert_eq!(embed(&response)["description"], "Saved");
    assert_eq!(embed(&response)["author"]["name"], "Bot");

    let tools = interaction(
        2,
        json!({ "id": "1", "name": "tools", "type": 1, "options": [{ "name": "info", "type": 1 }] }),
    );
    let response = respond(&Themed, tools);
    assert_eq!(embed(&response)["description"], "nested");
    assert_eq!(embed(&response)["footer"]["text"], "Footer");

    let click = interaction(3, json!({ "custom_id": "warn", "component_type": 2 }));
    let response = respond(&Themed, click);
    assert_eq!(embed(&response)["color"], Theme::DEFAULT.warning);
    assert_eq!(embed(&response)["author"]["name"], "Bot");
}

#[test]
fn unthemed_responses_and_text_are_left_alone() {
    let response = respond(&Themed, chat("raw"));
    assert_eq!(embed(&response)["color"], Value::Null);
    assert_eq!(embed(&response)["author"], Value::Null);
    assert_eq!(response["data"]["flags"], 64);

    let response = respond(&Themed, chat("text"));
    assert_eq!(response["data"]["content"], "hi");
    assert!(response["data"].get("embeds").is_none());
}

#[test]
fn errors_are_themed_embeds() {
    let response = respond(&Themed, chat("refuse"));
    assert_eq!(response["data"]["flags"], 64);
    assert_eq!(embed(&response)["color"], Theme::DEFAULT.error);
    assert_eq!(embed(&response)["description"], "Not allowed");
    assert_eq!(embed(&response)["footer"]["text"], "Footer");

    let response = respond(&Themed, chat("nope"));
    assert_eq!(embed(&response)["description"], "Unknown command.");
}

#[test]
fn sets_without_a_theme_are_unchanged() {
    use unthemed::Unthemed;

    assert_eq!(InteractionHandler::<()>::theme(&Unthemed), None);
    let response = respond(&Unthemed, chat("plain"));
    assert_eq!(embed(&response)["color"], Value::Null);

    // Helpers still work, with the default palette and no branding
    let response = respond(&Unthemed, chat("saved"));
    assert_eq!(embed(&response)["color"], Theme::DEFAULT.success);
    assert_eq!(embed(&response)["author"], Value::Null);

    let response = respond(&Unthemed, chat("refuse"));
    assert_eq!(response["data"]["content"], "Not allowed");
}

#[test]
fn fallback_keeps_the_sets_theme() {
    let handler = Themed.with_fallback(composure::command::NoHandler);
    assert_eq!(InteractionHandler::<()>::theme(&handler), Some(&THEME));
    let response = respond(&handler, chat("plain"));
    assert_eq!(embed(&response)["color"], 0xf5c2e7);
}

#[test]
fn theme_can_be_an_inline_expression() {
    let response = respond(&inline::Inline, chat("plain"));
    assert_eq!(embed(&response)["color"], 0x123456);
}
