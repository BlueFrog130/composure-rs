//! What the Workers bot's `sync` bin publishes: the shared commands, then `/note`

use composure::command::CommandSetDefinition;
use composure_example_bot::Bot;
use composure_example_cloudflare::CloudflareBot;
use serde_json::{Value, json};

#[test]
fn shared_commands_then_note() {
    let shared = Bot::definitions().len();
    let definitions = CloudflareBot::definitions();
    assert_eq!(definitions.len(), shared + 1);

    let note: Value = serde_json::to_value(&definitions[shared].command).unwrap();
    assert_eq!(note["name"], "note");
    let subcommands: Vec<_> = note["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["name"].clone())
        .collect();
    assert_eq!(subcommands, [json!("save"), json!("show"), json!("forget")]);
}
