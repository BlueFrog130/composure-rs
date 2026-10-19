//! Views: `rsx!` markup expansion, `#[view]` Props, inline handlers and their routing.

use composure::{
    Choice,
    command::{Ephemeral, HandlerError, IntoResponse, ModalView, Update, View, dispatch},
    commands, component, rsx,
    types::{Interaction, InteractionResponse},
    view,
};
use futures::executor::block_on;
use serde_json::{Value, json};

#[component("count")]
fn count(n: u32) -> Update<String> {
    Update(format!("{}", n + 1))
}

#[derive(Choice, Debug, PartialEq)]
enum Color {
    Red,
    Green,
    #[choice(name = "Deep blue")]
    Blue,
}

/// Clicks counted in the custom_id
#[view]
pub fn Counter(count: u32, #[prop(default = 1)] step: u32, note: Option<String>) -> View {
    rsx! {
        <Text>"Clicked " {count} " times" {note.as_deref().map(|n| format!(" ({n})"))}</Text>
        <Row>
            <Button primary on:click:add=move |_| Counter { count: count + step, step, note }>
                "+" {step}
            </Button>
            <Button danger on:click:reset=move |_| Ephemeral("reset")>"Reset"</Button>
        </Row>
    }
}

/// A card with a list of rows, each a sub-View with its own handler
#[view(prefix = "c")]
pub fn Card(title: String, items: Vec<u32>) -> View {
    rsx! {
        <Container accent_color=0x00ff00 spoiler>
            "## " {&title}
            <Separator large divider=false/>
            {items.iter().map(|&id| rsx! { <ItemRow id=id/> })}
            <Section>
                <Text>"Docs"</Text>
                <Button url="https://example.com">"Open"</Button>
            </Section>
            <Gallery>
                <Item url="https://example.com/a.png" description="a"/>
            </Gallery>
        </Container>
    }
}

#[view]
pub fn ItemRow(id: u32) -> View {
    rsx! {
        <Section>
            "Item " {id}
            <Button on:click:remove=move |_| Ephemeral(format!("removed {id}"))>"Remove"</Button>
        </Section>
    }
}

/// Selects: typed values from a Choice enum
#[view]
pub fn Palette(picked: Option<String>) -> View {
    rsx! {
        <Text>"Picked: " {picked.as_deref().unwrap_or("nothing")}</Text>
        <Row>
            <StringSelect placeholder="Colors" options=Color
                on:select:pick={move |_ctx, colors: Vec<Color>| Palette {
                    picked: Some(format!("{colors:?}")),
                }}
            />
        </Row>
    }
}

/// A modal View opened from a button, read back as typed parameters
#[view]
pub fn Feedback(topic: String) -> ModalView {
    rsx! {
        <Modal title="Feedback" on:submit:send={move |_ctx, details: String, color: Option<Color>, contact: bool| {
            Ephemeral(format!("{topic}: {details} {color:?} {contact}"))
        }}>
            <Text>"About " {&topic}</Text>
            <Label text="Details" description="Tell us">
                <TextInput name="details" paragraph max_len=100/>
            </Label>
            <Label text="Color">
                <RadioGroup name="color" options=Color/>
            </Label>
            <Label text="Contact me">
                <Checkbox name="contact"/>
            </Label>
        </Modal>
    }
}

#[view]
pub fn Opener() -> View {
    rsx! {
        <Row>
            <Button on:click:open=move |_| Feedback { topic: "bugs".to_string() }>"Open"</Button>
            <Button on:click=count(41)>"Count"</Button>
        </Row>
    }
}

/// Wraps its children
#[view]
pub fn Panel(title: String, children: View) -> View {
    rsx! {
        <Container>
            "### " {title}
            {children}
        </Container>
    }
}

/// A counter
#[composure::command]
fn counter() -> Counter {
    Counter::builder().count(0).build()
}

commands! {
    pub struct Bot = [counter];
    components = [count];
    views = [Counter, Card, ItemRow, Palette, Feedback, Opener];
}

fn json(response: InteractionResponse) -> Value {
    serde_json::to_value(response).unwrap()
}

fn interaction(t: u8, data: Value) -> Interaction {
    serde_json::from_value(json!({
        "id": "1", "application_id": "2", "type": t, "token": "t", "version": 1,
        "user": { "id": "3", "username": "ana", "discriminator": "0", "global_name": null, "avatar": null },
        "data": data,
    }))
    .unwrap()
}

fn click(custom_id: &str, values: Option<Vec<&str>>) -> Value {
    let mut data = json!({ "custom_id": custom_id, "component_type": 2 });
    if let Some(values) = values {
        data["component_type"] = json!(3);
        data["values"] = json!(values);
    }
    let interaction = interaction(3, data);
    json(block_on(dispatch(interaction, &Bot, &())).unwrap())
}

fn custom_ids(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(id)) = map.get("custom_id") {
                out.push(id.clone());
            }
            map.values().for_each(|v| custom_ids(v, out));
        }
        Value::Array(items) => items.iter().for_each(|v| custom_ids(v, out)),
        _ => {}
    }
}

fn ids(value: &Value) -> Vec<String> {
    let mut out = Vec::new();
    custom_ids(value, &mut out);
    out
}

#[test]
fn static_markup_serializes() {
    let view = rsx! {
        <Container accent_color=0x123456>
            "## Hi " {3}
            <Row>
                <Button success custom_id="x" disabled emoji="👍">"Go"</Button>
            </Row>
        </Container>
    };
    assert_eq!(
        json(view.into_response()),
        json!({ "type": 4, "data": { "flags": 1 << 15, "components": [{
            "type": 17,
            "accent_color": 0x123456,
            "components": [
                { "type": 10, "content": "## Hi 3" },
                { "type": 1, "components": [{
                    "type": 2, "style": 3, "label": "Go", "custom_id": "x", "disabled": true,
                    "emoji": { "id": null, "name": "👍", "animated": null },
                }]},
            ],
        }]}})
    );
}

#[test]
fn command_sends_view_with_inline_handler_ids() {
    let interaction = interaction(2, json!({ "id": "9", "name": "counter", "type": 1 }));
    let response = json(block_on(dispatch(interaction, &Bot, &())).unwrap());
    assert_eq!(response["type"], 4);
    assert_eq!(
        response["data"]["components"][0]["content"],
        "Clicked 0 times"
    );
    // `add` captures count, step and note (`None` is an empty field); `reset` nothing
    assert_eq!(ids(&response), ["counter:add:0:1:", "counter:reset"]);
}

#[test]
fn inline_handler_rerenders_with_new_props() {
    let response = click("counter:add:4:2:hi", None);
    assert_eq!(response["type"], 7, "an update of the message");
    assert_eq!(
        response["data"]["components"][0]["content"],
        "Clicked 6 times (hi)"
    );
    assert_eq!(ids(&response), ["counter:add:6:2:hi", "counter:reset"]);

    let reset = click("counter:reset", None);
    assert_eq!(reset["type"], 4);
    assert_eq!(reset["data"]["content"], "reset");
}

#[test]
fn optional_props_and_text_joining() {
    let view = Counter::builder()
        .count(1)
        .note("hey")
        .step(5)
        .build()
        .render();
    let value = json(view.into_response());
    assert_eq!(
        value["data"]["components"][0]["content"],
        "Clicked 1 times (hey)"
    );
    assert_eq!(
        value["data"]["components"][1]["components"][0]["label"],
        "+5"
    );
}

#[test]
fn sub_views_route_their_own_handlers() {
    let card = Card::builder()
        .title("Things")
        .items(vec![7, 8])
        .build()
        .render();
    let value = json(card.into_response());
    let container = &value["data"]["components"][0];
    assert_eq!(container["accent_color"], 0x00ff00);
    assert_eq!(container["spoiler"], true);
    assert_eq!(container["components"][0]["content"], "## Things");
    assert_eq!(
        container["components"][1],
        json!({ "type": 14, "divider": false, "spacing": 2 })
    );
    assert_eq!(
        container["components"][2]["components"][0]["content"],
        "Item 7"
    );
    assert_eq!(
        container["components"][4]["accessory"]["url"],
        "https://example.com"
    );
    assert_eq!(container["components"][4]["accessory"]["style"], 5);
    assert_eq!(
        container["components"][5]["items"][0]["media"]["url"],
        "https://example.com/a.png"
    );
    assert_eq!(ids(&value), ["item_row:remove:7", "item_row:remove:8"]);

    assert_eq!(
        click("item_row:remove:8", None)["data"]["content"],
        "removed 8"
    );
}

#[test]
fn select_values_are_typed() {
    let value = json(Palette::builder().build().into_response());
    let select = &value["data"]["components"][1]["components"][0];
    assert_eq!(select["custom_id"], "palette:pick");
    assert_eq!(
        select["options"][2],
        json!({ "label": "Deep blue", "value": "Blue" })
    );
    assert_eq!(select["max_values"], 3, "a Vec allows every option");

    let picked = click("palette:pick", Some(vec!["Red", "Blue"]));
    assert_eq!(picked["type"], 7);
    assert_eq!(
        picked["data"]["components"][0]["content"],
        "Picked: [Red, Blue]"
    );
}

#[test]
fn modal_view_opens_and_reads_typed_fields() {
    let opened = click("opener:open", None);
    assert_eq!(opened["type"], 9);
    let data = &opened["data"];
    assert_eq!(data["custom_id"], "feedback:send:bugs");
    assert_eq!(data["title"], "Feedback");
    assert_eq!(data["components"][0]["content"], "About bugs");
    let details = &data["components"][1];
    assert_eq!(details["label"], "Details");
    assert_eq!(details["description"], "Tell us");
    assert_eq!(details["component"]["custom_id"], "details");
    assert_eq!(details["component"]["style"], 2);
    assert_eq!(details["component"]["max_length"], 100);
    assert!(
        details["component"].get("required").is_none(),
        "String is required"
    );
    assert_eq!(
        data["components"][2]["component"]["required"], false,
        "Option is not"
    );

    let submit = interaction(
        5,
        json!({ "custom_id": "feedback:send:bugs", "components": [
            { "type": 18, "component": { "type": 4, "custom_id": "details", "value": "broken" } },
            { "type": 18, "component": { "type": 21, "custom_id": "color", "value": "Green" } },
            { "type": 18, "component": { "type": 23, "custom_id": "contact", "value": true } },
        ]}),
    );
    let response = json(block_on(dispatch(submit, &Bot, &())).unwrap());
    assert_eq!(response["data"]["content"], "bugs: broken Some(Green) true");
}

#[test]
fn reference_handlers_use_their_custom_id() {
    let value = json(Opener::builder().build().into_response());
    assert_eq!(ids(&value), ["opener:open", "count:41"]);
    assert_eq!(click("count:41", None)["data"]["content"], "42");
}

#[test]
fn children_prop_takes_markup() {
    let view = rsx! {
        <Panel title="Box">
            <Text>"inside"</Text>
        </Panel>
    };
    let value = json(view.into_response());
    let container = &value["data"]["components"][0];
    assert_eq!(container["components"][0]["content"], "### Box");
    assert_eq!(container["components"][1]["content"], "inside");
}

#[test]
fn unknown_handler_names_fall_through() {
    let response = click("counter:gone", None);
    assert_eq!(
        response["data"]["flags"], 64,
        "the ephemeral not-handled message"
    );
}

#[test]
fn too_long_custom_id_is_an_error() {
    let result = Feedback::builder()
        .topic("y".repeat(120))
        .build()
        .try_into_response();
    let error: HandlerError = result.unwrap_err();
    assert!(error.to_string().contains("custom_id is"), "{error}");
}
