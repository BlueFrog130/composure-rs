//! `#[derive(Modal)]` with every input kind: the modal it shows and the values it reads back.

use composure::{
    Choice, Modal,
    command::Modal as _,
    types::{Attachment, ModalSubmitData, PartialChannel, Role, Snowflake, User},
};
use serde_json::{Value, json};

#[derive(Choice, Debug, PartialEq)]
enum Severity {
    Low,
    #[choice(name = "Really bad", value = "high")]
    High,
}

#[derive(Choice, Debug, PartialEq)]
enum Size {
    #[choice(value = 1)]
    Small,
    #[choice(value = 10)]
    Large,
}

#[derive(Modal, Debug)]
#[modal(title = "Bug report", text = "Thanks for **helping**!")]
struct BugReport {
    #[input(max_len = 100, placeholder = "One line")]
    summary: String,
    #[radio(description = "How bad is it?")]
    severity: Severity,
    #[checkboxes(
        option("Windows", "win"),
        option("macOS", "mac"),
        option("Linux", "linux")
    )]
    platforms: Vec<String>,
    #[file_upload(max = 3, file_types("image", ".log"))]
    screenshots: Vec<Attachment>,
}

#[derive(Modal, Debug)]
#[modal(title = "Settings")]
struct Settings {
    #[select(placeholder = "Pick a size")]
    size: Option<Size>,
    #[user_select(label = "Moderator")]
    moderator: User,
    #[role_select(min = 1, max = 3)]
    roles: Vec<Role>,
    #[channel_select(channel_types(GuildText))]
    channel: Option<PartialChannel>,
    #[checkbox(label = "Notify me", default)]
    notify: bool,
}

#[derive(Modal, Debug)]
#[modal(title = "Raw")]
struct Raw {
    #[mentionable_select(id = "who")]
    mentioned: Vec<Snowflake>,
    #[select(option("A", "a"), option("B", "b"), max = 2)]
    letters: Vec<String>,
    #[radio(option("Yes", "y"), option("No", "n"))]
    answer: Option<String>,
}

fn components<M: composure::command::Modal>() -> Value {
    serde_json::to_value(M::components()).unwrap()
}

fn submit(components: Value, resolved: Value) -> ModalSubmitData {
    serde_json::from_value(json!({
        "custom_id": "m", "components": components, "resolved": resolved,
    }))
    .unwrap()
}

fn label(child: Value) -> Value {
    json!({ "type": 18, "id": 1, "component": child })
}

#[test]
fn every_input_kind_renders_in_a_label() {
    assert_eq!(
        components::<BugReport>(),
        json!([
            { "type": 10, "content": "Thanks for **helping**!" },
            { "type": 18, "label": "Summary", "component": {
                "type": 4, "custom_id": "summary", "style": 1, "max_length": 100,
                "placeholder": "One line",
            } },
            { "type": 18, "label": "Severity", "description": "How bad is it?", "component": {
                "type": 21, "custom_id": "severity", "options": [
                    { "label": "Low", "value": "Low" },
                    { "label": "Really bad", "value": "high" },
                ],
            } },
            { "type": 18, "label": "Platforms", "component": {
                "type": 22, "custom_id": "platforms", "required": false, "options": [
                    { "label": "Windows", "value": "win" },
                    { "label": "macOS", "value": "mac" },
                    { "label": "Linux", "value": "linux" },
                ],
            } },
            { "type": 18, "label": "Screenshots", "component": {
                "type": 19, "custom_id": "screenshots", "max_values": 3, "required": false,
                "file_types": ["image", ".log"],
            } },
        ])
    );
    assert_eq!(
        components::<Settings>(),
        json!([
            { "type": 18, "label": "Size", "component": {
                "type": 3, "custom_id": "size", "placeholder": "Pick a size", "required": false,
                "options": [{ "label": "Small", "value": "1" }, { "label": "Large", "value": "10" }],
            } },
            { "type": 18, "label": "Moderator", "component": { "type": 5, "custom_id": "moderator" } },
            { "type": 18, "label": "Roles", "component": {
                "type": 6, "custom_id": "roles", "min_values": 1, "max_values": 3, "required": false,
            } },
            { "type": 18, "label": "Channel", "component": {
                "type": 8, "custom_id": "channel", "channel_types": [0], "required": false,
            } },
            { "type": 18, "label": "Notify me", "component": {
                "type": 23, "custom_id": "notify", "default": true,
            } },
        ])
    );
    let raw = components::<Raw>();
    assert_eq!(raw[0]["component"]["custom_id"], "who");
    assert_eq!(raw[0]["component"]["max_values"], 25, "Vec allows many");
    assert_eq!(raw[1]["component"]["max_values"], 2);
    assert_eq!(BugReport::TITLE, "Bug report");
}

#[test]
fn submitted_values_are_typed() {
    let data = submit(
        json!([
            { "type": 10, "id": 0 },
            label(json!({ "type": 4, "custom_id": "summary", "value": "Crash" })),
            label(json!({ "type": 21, "custom_id": "severity", "value": "high" })),
            label(json!({ "type": 22, "custom_id": "platforms", "values": ["mac", "linux"] })),
            label(json!({ "type": 19, "custom_id": "screenshots", "values": ["900"] })),
        ]),
        json!({ "attachments": { "900": {
            "id": "900", "filename": "crash.png", "size": 10, "url": "https://x/crash.png",
            "proxy_url": "https://x/crash.png",
        } } }),
    );
    let report = BugReport::extract(&data).unwrap();
    assert_eq!(report.summary, "Crash");
    assert_eq!(report.severity, Severity::High);
    assert_eq!(report.platforms, ["mac", "linux"]);
    assert_eq!(report.screenshots[0].filename, "crash.png");

    let user = json!({ "id": "5", "username": "mod", "discriminator": "0", "avatar": null });
    let data = submit(
        json!([
            label(json!({ "type": 3, "custom_id": "size", "values": ["10"] })),
            label(json!({ "type": 5, "custom_id": "moderator", "values": ["5"] })),
            label(json!({ "type": 6, "custom_id": "roles", "values": [] })),
            label(json!({ "type": 8, "custom_id": "channel", "values": [] })),
            label(json!({ "type": 23, "custom_id": "notify", "value": true })),
        ]),
        json!({ "users": { "5": user } }),
    );
    let settings = Settings::extract(&data).unwrap();
    assert_eq!(settings.size, Some(Size::Large));
    assert_eq!(settings.moderator.username, "mod");
    assert!(settings.roles.is_empty());
    assert!(settings.channel.is_none());
    assert!(settings.notify);

    let data = submit(
        json!([
            label(json!({ "type": 7, "custom_id": "who", "values": ["1", "2"] })),
            label(json!({ "type": 3, "custom_id": "letters", "values": ["b"] })),
            label(json!({ "type": 21, "custom_id": "answer", "value": null })),
        ]),
        Value::Null,
    );
    let raw = Raw::extract(&data).unwrap();
    assert_eq!(raw.mentioned.len(), 2);
    assert_eq!(raw.letters, ["b"]);
    assert_eq!(
        raw.answer, None,
        "nothing picked in an optional radio group"
    );
}

#[test]
fn bad_submissions_are_errors() {
    // A required field left out
    let data = submit(json!([]), Value::Null);
    let error = BugReport::extract(&data).unwrap_err();
    assert!(error.to_string().contains("summary"), "{error}");

    // An unresolved user
    let data = submit(
        json!([label(
            json!({ "type": 5, "custom_id": "moderator", "values": ["5"] })
        ),]),
        Value::Null,
    );
    assert!(Settings::extract(&data).is_err());

    // Not one of the choices
    let data = submit(
        json!([
            label(json!({ "type": 4, "custom_id": "summary", "value": "x" })),
            label(json!({ "type": 21, "custom_id": "severity", "value": "nope" })),
        ]),
        Value::Null,
    );
    let error = BugReport::extract(&data).unwrap_err();
    assert!(error.to_string().contains("severity"), "{error}");
}

#[test]
fn unchecked_checkboxes_are_false() {
    let user = json!({ "id": "5", "username": "mod", "discriminator": "0", "avatar": null });
    let data = submit(
        json!([
            label(json!({ "type": 5, "custom_id": "moderator", "values": ["5"] })),
            label(json!({ "type": 23, "custom_id": "notify", "value": false })),
        ]),
        json!({ "users": { "5": user } }),
    );
    assert!(!Settings::extract(&data).unwrap().notify);
}
