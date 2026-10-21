//! `#[app(errors = ..)]`: the response handler errors are answered with, for the initial
//! response and for deferred work, forwarded by the root from a nested App and kept by
//! `with_fallback`.

use composure::{
    Ctx, app,
    command::{Deferred, HandlerError, InteractionHandler, NoHandler, Platform},
    testing::{self, ResponseExt, TestPlatform},
    types::{Component, Container, InteractionResponse},
};

/// Fails at once
#[composure::command]
fn refuse() -> Result<String, HandlerError> {
    Err(HandlerError::user("Not allowed"))
}

/// Fails in Background Work, after the deferral
#[composure::command]
fn later(ctx: Ctx<impl Platform>) -> Deferred {
    ctx.defer(async |_| Err::<String, _>(HandlerError::user("Too late")))
}

/// Errors as a red Components V2 message
fn render(error: &HandlerError) -> InteractionResponse {
    InteractionResponse::respond_with_components(vec![
        Container::new(vec![Component::text(error.user_message())])
            .with_accent_color(0xf38ba8)
            .into(),
    ])
}

mod module {
    use super::*;

    #[app(handlers(refuse, later), errors = render)]
    pub struct Module;
}

/// The root sets nothing; the module's renderer is the tree's
#[app(handlers(module::Module))]
struct Bot;

/// A closure works too
#[app(handlers(refuse), errors = |error| InteractionResponse::respond_with_message(format!("closure: {error}")))]
struct Inline;

#[app(handlers(refuse))]
struct Plain;

fn accent(response: &impl ResponseExt) -> serde_json::Value {
    response.components()[0]["accent_color"].clone()
}

#[test]
fn the_initial_response_is_rendered_by_the_hook() {
    let response = testing::answer(&Bot, testing::command("refuse"), &TestPlatform::new());
    assert_eq!(accent(&response), 0xf38ba8);
    assert_eq!(response.text().as_deref(), Some("Not allowed"));

    let error = testing::run(&Bot, testing::command("refuse"), &TestPlatform::new()).unwrap_err();
    assert_eq!(
        InteractionHandler::<TestPlatform>::render_error(&Bot, &error).json(),
        response.json(),
        "what Adapters call"
    );
}

#[test]
fn deferred_work_errors_are_rendered_by_the_hook() {
    let platform = TestPlatform::new();
    let response = testing::answer(&Bot, testing::command("later"), &platform);
    assert_eq!(response.response_type(), 5);
    assert!(platform.run_background().is_err());
    let edit = platform.transport().body(0);
    assert_eq!(edit["components"][0]["accent_color"], 0xf38ba8);
    assert_eq!(
        edit["components"][0]["components"][0]["content"],
        "Too late"
    );
}

#[test]
fn fallback_keeps_the_hook() {
    let handler = Bot.with_fallback(NoHandler);
    let response = testing::answer(&handler, testing::command("refuse"), &TestPlatform::new());
    assert_eq!(accent(&response), 0xf38ba8);
}

#[test]
fn closures_and_the_default() {
    let response = testing::answer(&Inline, testing::command("refuse"), &TestPlatform::new());
    assert_eq!(response.text().as_deref(), Some("closure: Not allowed"));

    let response = testing::answer(&Plain, testing::command("refuse"), &TestPlatform::new());
    assert_eq!(response.text().as_deref(), Some("Not allowed"));
    assert!(response.is_ephemeral());
}
