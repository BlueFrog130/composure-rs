//! Autocomplete Handlers attached to Options: Definitions and routing the focused Option.

use composure::{
    Ctx, app, autocomplete,
    command::{CommandSetDefinition, HandlerError, dispatch},
    group,
    types::Interaction,
};
use futures::executor::block_on;
use serde_json::{Value, json};

const FRUITS: [&str; 4] = ["apple", "apricot", "banana", "cherry"];

#[autocomplete]
fn fruits(input: &str) -> Vec<&'static str> {
    FRUITS
        .into_iter()
        .filter(|fruit| fruit.starts_with(input))
        .collect()
}

/// Multiples of the State, as `(name, value)` pairs
#[autocomplete]
async fn multiples(ctx: Ctx<u32>, input: &str) -> Result<Vec<(String, i64)>, HandlerError> {
    let base: i64 = input
        .parse()
        .map_err(|_| HandlerError::user("type a number"))?;
    Ok((1..=3)
        .map(|n| (format!("{base} x {n}"), base * n * i64::from(*ctx.state())))
        .collect())
}

/// Pick a fruit
#[composure::command]
fn fruit(
    #[option("Fruit", autocomplete = fruits)] name: &str,
    #[option("How many")] count: Option<u8>,
) -> String {
    format!("{} {name}", count.unwrap_or(1))
}

/// Pick a multiple
#[composure::command]
fn multiply(_ctx: Ctx<u32>, #[option("Number", autocomplete = multiples)] n: i64) -> String {
    n.to_string()
}

/// Shop tools
#[group(subcommands(fruit))]
struct Shop;

#[app(handlers(fruit, multiply, Shop))]
pub struct Bot;

fn autocomplete_interaction(data: Value) -> Interaction {
    serde_json::from_value(json!({
        "id": "1", "application_id": "2", "type": 4, "token": "t", "version": 1,
        "user": { "id": "3", "username": "ana", "discriminator": "0", "global_name": null, "avatar": null },
        "data": data,
    }))
    .unwrap()
}

fn suggest(data: Value) -> Result<Value, HandlerError> {
    block_on(dispatch(autocomplete_interaction(data), &Bot, &10u32))
        .map(|response| serde_json::to_value(response).unwrap())
}

#[test]
fn focused_option_reaches_its_handler() {
    let response = suggest(json!({
        "id": "9", "name": "fruit", "type": 1,
        "options": [{ "name": "name", "type": 3, "value": "ap", "focused": true }],
    }))
    .unwrap();
    assert_eq!(
        response,
        json!({ "type": 8, "data": { "choices": [
            { "name": "apple", "value": "apple" },
            { "name": "apricot", "value": "apricot" },
        ] } })
    );
}

#[test]
fn handlers_get_ctx_and_can_fail() {
    let response = suggest(json!({
        "id": "9", "name": "multiply", "type": 1,
        "options": [{ "name": "n", "type": 4, "value": "2", "focused": true }],
    }))
    .unwrap();
    assert_eq!(
        response["data"]["choices"][2],
        json!({ "name": "2 x 3", "value": 60 })
    );

    let error = suggest(json!({
        "id": "9", "name": "multiply", "type": 1,
        "options": [{ "name": "n", "type": 4, "value": "x", "focused": true }],
    }))
    .unwrap_err();
    assert_eq!(error.to_string(), "type a number");
}

#[test]
fn groups_route_to_the_subcommand() {
    let response = suggest(json!({
        "id": "9", "name": "shop", "type": 1,
        "options": [{ "name": "fruit", "type": 1, "options": [
            { "name": "name", "type": 3, "value": "b", "focused": true },
        ] }],
    }))
    .unwrap();
    assert_eq!(
        response["data"]["choices"],
        json!([{ "name": "banana", "value": "banana" }])
    );
}

#[test]
fn options_without_a_handler_get_no_choices() {
    let response = suggest(json!({
        "id": "9", "name": "fruit", "type": 1,
        "options": [{ "name": "count", "type": 4, "value": "1", "focused": true }],
    }))
    .unwrap();
    assert_eq!(response, json!({ "type": 8, "data": { "choices": [] } }));
}

#[test]
fn definitions_enable_autocomplete() {
    let definitions = Bot::definitions();
    let fruit = serde_json::to_value(&definitions[0].command).unwrap();
    assert_eq!(fruit["options"][0]["autocomplete"], true);
    assert_eq!(fruit["options"][1].get("autocomplete"), None);
    let multiply = serde_json::to_value(&definitions[1].command).unwrap();
    assert_eq!(multiply["options"][0]["autocomplete"], true);
}
