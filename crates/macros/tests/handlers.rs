//! Component and Modal Handlers defined with the macros: custom_id round trips and routing.

use composure::{
    Ctx, Modal, app,
    command::{
        ComponentCtx, Ephemeral, HandlerError, Modal as _, ModalCtx, Update,
        custom_id::{CustomIdError, split},
        dispatch,
    },
    component, modal,
    types::{Interaction, InteractionResponse},
};
use futures::executor::block_on;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Serialize, Deserialize)]
enum Direction {
    Up,
    Down,
}

/// Counts clicks; the count lives in the custom_id
#[component("count")]
fn count(n: u32) -> Update<String> {
    Update(format!("{}", n + 1))
}

/// Moves in a direction, with the State and the invoking user
#[component("move")]
async fn step(ctx: ComponentCtx<u32>, direction: Direction, by: Option<i8>) -> String {
    let who = ctx.user().map_or("nobody", |u| u.username.as_str());
    format!("{who} {direction:?} {:?} state {}", by, ctx.state())
}

/// No state at all
#[component("plain")]
fn plain(ctx: ComponentCtx) -> Ephemeral<String> {
    Ephemeral(format!("values {:?}", ctx.values()))
}

#[derive(Modal, Debug)]
#[modal(title = "Rename")]
struct Rename {
    #[input(label = "New name", max_len = 32, placeholder = "name")]
    name: String,
    #[input(paragraph)]
    reason_why: Option<String>,
}

/// Renames item `item`
#[modal("rename")]
fn rename(ctx: ModalCtx, form: Rename, item: u32) -> Result<String, HandlerError> {
    if form.name == "bad" {
        return Err(HandlerError::user("Not that name."));
    }
    let raw = ctx.value("name").unwrap_or_default();
    Ok(format!(
        "item {item} -> {} ({raw}) because {:?}",
        form.name, form.reason_why
    ))
}

/// Opens the rename form
#[composure::command]
fn edit(#[option("Item")] item: u32) -> Result<InteractionResponse, CustomIdError> {
    rename::show(item)
}

/// Unused here; shows a command and handlers share a State
#[composure::command]
fn noop(_ctx: Ctx<u32>) -> &'static str {
    ""
}

struct Events;

impl composure::command::EventHandler<u32> for Events {
    async fn unknown(
        &self,
        ctx: composure::command::EventCtx<'_, u32>,
        kind: &str,
        _data: Option<&serde_json::value::RawValue>,
    ) -> Result<(), HandlerError> {
        Err(HandlerError::new(format!(
            "{kind} with state {}",
            ctx.state()
        )))
    }
}

#[app(handlers(edit, noop, count, step, plain, rename), events = Events)]
pub struct Bot;

/// No `events`: every event is ignored
mod quiet {
    use super::*;

    #[app(handlers(noop))]
    pub struct Quiet;
}

fn interaction(t: u8, data: Value) -> Interaction {
    serde_json::from_value(json!({
        "id": "1", "application_id": "2", "type": t, "token": "t", "version": 1,
        "user": { "id": "3", "username": "ana", "discriminator": "0", "global_name": null, "avatar": null },
        "data": data,
    }))
    .unwrap()
}

fn click(custom_id: &str) -> Value {
    let interaction = interaction(3, json!({ "custom_id": custom_id, "component_type": 2 }));
    serde_json::to_value(block_on(dispatch(interaction, &Bot, &7u32)).unwrap()).unwrap()
}

fn submit(custom_id: &str, name: &str, reason: &str) -> Result<Value, HandlerError> {
    let interaction = interaction(
        5,
        json!({ "custom_id": custom_id, "components": [
            { "type": 1, "components": [{ "type": 4, "custom_id": "name", "value": name }] },
            { "type": 1, "components": [{ "type": 4, "custom_id": "reason_why", "value": reason }] },
        ] }),
    );
    block_on(dispatch(interaction, &Bot, &7u32)).map(|r| serde_json::to_value(r).unwrap())
}

#[test]
fn custom_ids_carry_state() {
    assert_eq!(count::custom_id(41).unwrap(), "count:41");
    assert_eq!(
        step::custom_id(Direction::Down, Some(-2)).unwrap(),
        "move:1:-2"
    );
    assert_eq!(step::custom_id(Direction::Up, None).unwrap(), "move:0:");
    assert_eq!(plain::custom_id().unwrap(), "plain");
    assert_eq!(split("count:41"), ("count", Some("41")));
}

#[test]
fn components_route_by_prefix() {
    let response = click(&count::custom_id(41).unwrap());
    assert_eq!(response, json!({ "type": 7, "data": { "content": "42" } }));

    let response = click(&step::custom_id(Direction::Down, Some(-2)).unwrap());
    assert_eq!(response["type"], 4);
    assert_eq!(response["data"]["content"], "ana Down Some(-2) state 7");

    let response = click("plain");
    assert_eq!(response["data"]["content"], "values []");
    assert_eq!(response["data"]["flags"], 64);

    // No handler: the default "not handled" message
    let response = click("other:1");
    assert_eq!(response["data"]["flags"], 64);
}

#[test]
fn bad_state_is_a_handler_error() {
    let interaction = interaction(3, json!({ "custom_id": "count:many", "component_type": 2 }));
    let error = block_on(dispatch(interaction, &Bot, &7u32)).unwrap_err();
    assert!(error.to_string().contains("`count`"), "{error}");
}

#[test]
fn modals_show_and_submit() {
    let edit = interaction(
        2,
        json!({ "id": "9", "name": "edit", "type": 1, "options": [{ "name": "item", "type": 4, "value": 5 }] }),
    );
    let response = serde_json::to_value(block_on(dispatch(edit, &Bot, &7u32)).unwrap()).unwrap();
    assert_eq!(
        response,
        json!({ "type": 9, "data": {
            "custom_id": "rename:5",
            "title": "Rename",
            "components": [
                { "type": 18, "label": "New name", "component": {
                    "type": 4, "custom_id": "name", "style": 1, "max_length": 32,
                    "placeholder": "name",
                } },
                { "type": 18, "label": "Reason why", "component": {
                    "type": 4, "custom_id": "reason_why", "style": 2, "required": false,
                } },
            ],
        } })
    );

    let response = submit("rename:5", "box", "").unwrap();
    assert_eq!(
        response["data"]["content"],
        "item 5 -> box (box) because None"
    );
    let response = submit("rename:5", "crate", "tidier").unwrap();
    assert_eq!(
        response["data"]["content"],
        "item 5 -> crate (crate) because Some(\"tidier\")"
    );
    assert_eq!(
        submit("rename:5", "bad", "").unwrap_err().to_string(),
        "Not that name."
    );
}

#[test]
fn modal_trait_is_usable_directly() {
    assert_eq!(Rename::TITLE, "Rename");
    assert_eq!(Rename::components().len(), 2);
}

#[test]
fn events_go_to_the_registered_handler() {
    use composure::{command::dispatch_event, types::EventBody};

    let body: EventBody = serde_json::from_value(json!({
        "type": "SOMETHING_NEW", "timestamp": "2024-10-18T14:42:53.064834",
    }))
    .unwrap();
    let app = composure::types::Snowflake::from_u64(1);
    let error = block_on(dispatch_event(&Bot, &7u32, app, &body)).unwrap_err();
    assert_eq!(error.to_string(), "SOMETHING_NEW with state 7");
    let error = block_on(dispatch_event(&Bot.with_fallback(()), &7u32, app, &body)).unwrap_err();
    assert_eq!(error.to_string(), "SOMETHING_NEW with state 7");
    assert!(block_on(dispatch_event(&quiet::Quiet, &7u32, app, &body)).is_ok());
}
