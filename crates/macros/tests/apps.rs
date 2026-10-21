//! Apps listing Apps: routing through nested Apps, `errors` and events from a nested App,
//! definitions of the whole tree.

use composure::{
    app,
    command::{
        CommandSetDefinition, ComponentCtx, EventCtx, EventHandler, HandlerError,
        InteractionHandler, NoHandler, View, dispatch, dispatch_event,
    },
    component, rsx,
    types::{Embed, EventBody, Interaction, InteractionResponse, Snowflake},
    view,
};
use futures::executor::block_on;
use serde_json::{Value, json};

/// Top-level
#[composure::command]
fn top() -> Embed {
    Embed::new().with_title("top")
}

/// In a nested App
#[composure::command]
fn deep() -> Embed {
    Embed::new().with_title("deep")
}

/// Fails, in a nested App
#[composure::command]
fn refuse() -> Result<String, HandlerError> {
    Err(HandlerError::user("Not allowed"))
}

#[component("poke")]
fn poke(_ctx: ComponentCtx) -> String {
    "poked".into()
}

#[view]
fn Counter(n: u32) -> View {
    rsx! { <Row><Button on:click:add=move |_| Counter { n: n + 1 }>"+"</Button></Row> }
}

struct Events;

impl<S: Sync> EventHandler<S> for Events {
    async fn unknown(
        &self,
        _ctx: EventCtx<'_, S>,
        kind: &str,
        _data: Option<&serde_json::value::RawValue>,
    ) -> Result<(), HandlerError> {
        Err(HandlerError::new(format!("inner got {kind}")))
    }
}

/// Errors as a public message, so the tests tell them from the default
fn render(error: &HandlerError) -> InteractionResponse {
    InteractionResponse::respond_with_message(format!("rendered: {}", error.user_message()))
}

/// A module's App: the bot's events and its error renderer
#[app(handlers(deep, refuse, poke, Counter), events = Events, errors = render)]
struct Inner;

/// The root lists the module's App next to its own Command
#[app(handlers(top, Inner))]
struct Outer;

/// An App without `errors` anywhere in its tree
#[app(handlers(top))]
struct Plain;

#[app(handlers(Plain))]
struct Wrapped;

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

fn click(custom_id: &str) -> Interaction {
    interaction(3, json!({ "custom_id": custom_id, "component_type": 2 }))
}

fn respond<H: InteractionHandler<()>>(handler: &H, interaction: Interaction) -> Value {
    serde_json::to_value(block_on(dispatch(interaction, handler, &())).unwrap()).unwrap()
}

#[test]
fn nested_apps_route_everything_they_list() {
    let response = respond(&Outer, chat("deep"));
    assert_eq!(response["data"]["embeds"][0]["title"], "deep");
    let response = respond(&Outer, click("poke"));
    assert_eq!(response["data"]["content"], "poked");
    let response = respond(&Outer, click(&format!("counter:add:{}", 1)));
    assert_eq!(response["type"], 7);

    let error = block_on(dispatch(chat("nope"), &Outer, &())).unwrap_err();
    assert_eq!(error.to_string(), "unknown command `nope`");
}

#[test]
fn the_root_renders_errors_with_the_trees_errors() {
    let error = block_on(dispatch(chat("refuse"), &Outer, &())).unwrap_err();
    let rendered =
        serde_json::to_value(InteractionHandler::<()>::render_error(&Outer, &error)).unwrap();
    assert_eq!(rendered["data"]["content"], "rendered: Not allowed");
    assert_eq!(rendered["data"].get("flags"), None);

    // The same renderer reaches the Ctx, for errors from deferred work
    assert!(<Outer as composure::command::RouteDefinition>::ERRORS.is_some());

    let rendered =
        serde_json::to_value(InteractionHandler::<()>::render_error(&Wrapped, &error)).unwrap();
    assert_eq!(rendered["data"]["content"], "Not allowed");
    assert_eq!(
        rendered["data"]["flags"], 64,
        "the default is ephemeral text"
    );
}

#[test]
fn events_reach_the_nested_app_that_sets_them() {
    let body: EventBody = serde_json::from_value(json!({
        "type": "SOMETHING_NEW", "timestamp": "2024-10-18T14:42:53.064834",
    }))
    .unwrap();
    let app = Snowflake::from_u64(1);
    let error = block_on(dispatch_event(&Outer, &(), app, &body)).unwrap_err();
    assert_eq!(error.to_string(), "inner got SOMETHING_NEW");
    assert!(block_on(dispatch_event(&Wrapped, &(), app, &body)).is_ok());
}

#[test]
fn definitions_cover_the_whole_tree() {
    let names: Vec<_> = Outer::definitions()
        .into_iter()
        .map(|d| serde_json::to_value(d.command).unwrap()["name"].clone())
        .collect();
    assert_eq!(names, ["top", "deep", "refuse"]);
}

#[test]
fn fallback_gets_what_nothing_in_the_tree_handles() {
    let handler = Outer.with_fallback(NoHandler);
    let response = respond(&handler, click("poke"));
    assert_eq!(response["data"]["content"], "poked");
    let response = respond(&handler, click("unknown"));
    assert_eq!(
        response["data"]["content"],
        "This bot does not handle components."
    );
}
