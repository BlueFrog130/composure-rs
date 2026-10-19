//! Regression fixtures through the full adapter pipeline (signature -> parse -> dispatch ->
//! response) on the host, via `handle_interaction`: every option type, subcommands/groups,
//! user/message commands, a user-installed command in a bot DM, autocomplete, components,
//! modals, handler errors, and tampered/missing signatures.

mod common;

use common::RegressionBot;
use composure::adapter::handle_interaction;
use futures::executor::block_on;

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
