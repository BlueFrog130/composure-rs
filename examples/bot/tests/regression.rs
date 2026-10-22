//! Regression fixtures through the full adapter pipeline (signature -> parse -> dispatch ->
//! response) on the host, via `handle_interaction`: every option type, subcommands/groups,
//! user/message commands, a user-installed command in a bot DM, autocomplete, components,
//! modals, handler errors, and tampered/missing signatures.

mod common;

use common::RegressionBot;
use composure::{adapter::handle_interaction, testing::block_on};

#[test]
fn signed_interactions() {
    let public_key = common::public_key();
    let mut failures = Vec::new();

    for case in common::cases() {
        let (signature, timestamp, body) = case.request();
        let outcome = block_on(handle_interaction(
            &public_key,
            signature.as_deref(),
            timestamp.as_deref(),
            &body,
            &RegressionBot,
            &(),
        ));

        if outcome.status() != case.status {
            failures.push(format!(
                "{}: expected {}, got {} ({outcome:?})",
                case.name,
                case.status,
                outcome.status()
            ));
            continue;
        }
        let response = outcome.response().map(|r| serde_json::to_value(r).unwrap());
        if response != case.response {
            failures.push(format!(
                "{}:\n  expected {}\n  got      {}",
                case.name,
                case.response.unwrap_or_default(),
                response.unwrap_or_default()
            ));
        }
    }

    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// A View with a `<File>` (the example's `/export`) through the signed pipeline: the response
/// carries the file, so Adapters upload it through the callback endpoint, with the message
/// pointing at the attachment
#[test]
fn view_with_a_file() {
    use composure::testing::{self, TestKey, TestPlatform};
    use composure_example_bot::{Bot, Config};
    use serde_json::json;

    let request = TestKey::default().signed(
        testing::command("export")
            .option("text", "hello file")
            .body(),
    );
    let state = TestPlatform::new().with_data(Config {
        greeting: "Howdy".into(),
    });
    let outcome = block_on(handle_interaction(
        &common::public_key(),
        Some(&request.signature),
        Some(&request.timestamp),
        &request.body,
        &Bot,
        &state,
    ));
    assert_eq!(outcome.status(), 200);
    let response = outcome.response().expect("a response");
    assert_eq!(response.files().len(), 1);
    assert_eq!(response.files()[0].bytes, b"hello file");
    assert_eq!(
        serde_json::to_value(response).unwrap(),
        json!({ "type": 4, "data": {
            "flags": (1 << 15) | (1 << 6),
            "components": [
                { "type": 10, "content": "Here's your file:" },
                { "type": 13, "file": { "url": "attachment://export.txt" } },
            ],
        } })
    );
}
