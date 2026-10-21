//! Checks every implemented Endpoint against Discord's OpenAPI spec (vendored at the commit in
//! `spec/PINNED`; bump with `scripts/update-spec.sh`) and reports what is not implemented yet,
//! leaving out what an HTTP-only bot has no use for ([`OUT_OF_SCOPE`]).

use std::collections::BTreeSet;

use super::Route;

/// Route tables per module; a route may appear in two modules (webhook execution and
/// interaction follow-ups share paths)
fn modules() -> Vec<&'static [Route]> {
    let mut modules = vec![super::interaction::ROUTES];
    #[cfg(feature = "rest-commands")]
    modules.push(super::commands::ROUTES);
    #[cfg(feature = "rest-messages")]
    modules.push(super::messages::ROUTES);
    #[cfg(feature = "rest-users")]
    modules.push(super::users::ROUTES);
    #[cfg(feature = "rest-channels")]
    modules.push(super::channels::ROUTES);
    #[cfg(feature = "rest-guilds")]
    modules.push(super::guilds::ROUTES);
    #[cfg(feature = "rest-members")]
    modules.push(super::members::ROUTES);
    #[cfg(feature = "rest-roles")]
    modules.push(super::roles::ROUTES);
    #[cfg(feature = "rest-bans")]
    modules.push(super::bans::ROUTES);
    #[cfg(feature = "rest-threads")]
    modules.push(super::threads::ROUTES);
    #[cfg(feature = "rest-invites")]
    modules.push(super::invites::ROUTES);
    #[cfg(feature = "rest-emojis")]
    modules.push(super::emojis::ROUTES);
    #[cfg(feature = "rest-stickers")]
    modules.push(super::stickers::ROUTES);
    #[cfg(feature = "rest-scheduled-events")]
    modules.push(super::scheduled_events::ROUTES);
    #[cfg(feature = "rest-automod")]
    modules.push(super::automod::ROUTES);
    #[cfg(feature = "rest-audit-log")]
    modules.push(super::audit_log::ROUTES);
    #[cfg(feature = "rest-webhooks")]
    modules.push(super::webhooks::ROUTES);
    #[cfg(feature = "rest-monetization")]
    modules.push(super::monetization::ROUTES);
    #[cfg(feature = "rest-polls")]
    modules.push(super::polls::ROUTES);
    #[cfg(feature = "rest-soundboard")]
    modules.push(super::soundboard::ROUTES);
    #[cfg(feature = "rest-application")]
    modules.push(super::application::ROUTES);
    #[cfg(feature = "rest-role-connections")]
    modules.push(super::role_connections::ROUTES);
    #[cfg(feature = "rest-onboarding")]
    modules.push(super::onboarding::ROUTES);
    #[cfg(feature = "rest-stage-instances")]
    modules.push(super::stage_instances::ROUTES);
    #[cfg(feature = "rest-lobbies")]
    modules.push(super::lobbies::ROUTES);
    modules
}

/// Spec routes Composure doesn't implement on purpose, by normalized path prefix (`{}` for
/// parameters), optionally after a method (`PUT /path`), with the reason
const OUT_OF_SCOPE: &[(&str, &str)] = &[
    (
        "/gateway",
        "the Gateway: HTTP-only bots don't connect to it",
    ),
    (
        "/oauth2/",
        "OAuth2: an app's own token info and user-token flows",
    ),
    (
        "/partner-sdk/",
        "the Social SDK's partner token exchange and DM moderation",
    ),
    (
        "/users/@me/connections",
        "a user's connections: needs their OAuth2 token",
    ),
    (
        "/users/@me/guilds/{}/member",
        "a user's own membership: needs their OAuth2 token",
    ),
    (
        "/users/@me/applications/{}/",
        "a user's role connection and entitlements: needs their OAuth2 token",
    ),
    (
        "/channels/{}/pins",
        "the deprecated pins Endpoints: `/channels/{}/messages/pins` replaces them",
    ),
    (
        "/channels/{}/recipients/",
        "group DM recipients: needs a user's OAuth2 token",
    ),
    (
        "/webhooks/{}/{}/github",
        "GitHub-compatible webhook payloads, posted by GitHub",
    ),
    (
        "/webhooks/{}/{}/slack",
        "Slack-compatible webhook payloads, posted by Slack",
    ),
    (
        "PUT /applications/{}/guilds/{}/commands/{}/permissions",
        "Command permissions: needs a user's OAuth2 token with `applications.commands.permissions.update`",
    ),
];

/// Whether `entry` (from [`OUT_OF_SCOPE`]) covers the normalized `route`
fn covers(entry: &str, (method, path): &(String, String)) -> bool {
    match entry.split_once(' ') {
        Some((entry_method, prefix)) => entry_method == method && path.starts_with(prefix),
        None => path.starts_with(entry),
    }
}

fn out_of_scope(route: &(String, String)) -> bool {
    OUT_OF_SCOPE.iter().any(|(entry, _)| covers(entry, route))
}

/// Parameter names differ between Endpoints sharing a path (`webhook_id` vs `application_id`),
/// so compare positions only
fn normalize(path: &str) -> String {
    let mut normalized = String::with_capacity(path.len());
    let mut in_param = false;
    for c in path.chars() {
        match c {
            '{' => {
                in_param = true;
                normalized.push_str("{}");
            }
            '}' => in_param = false,
            _ if in_param => {}
            c => normalized.push(c),
        }
    }
    normalized
}

fn spec_routes() -> BTreeSet<(String, String)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/spec/openapi.json");
    let spec = std::fs::read(path).expect("vendored spec at spec/openapi.json");
    let spec: serde_json::Value = serde_json::from_slice(&spec).expect("spec is JSON");
    let mut routes = BTreeSet::new();
    for (path, item) in spec["paths"].as_object().expect("spec has paths") {
        for method in ["get", "post", "put", "patch", "delete"] {
            if item.get(method).is_some() {
                routes.insert((method.to_uppercase(), normalize(path)));
            }
        }
    }
    routes
}

fn implemented() -> Vec<Route> {
    modules().into_iter().flatten().copied().collect()
}

fn key(route: &Route) -> (String, String) {
    (route.method.as_str().to_string(), normalize(route.path))
}

#[test]
fn implemented_routes_exist_in_spec() {
    let spec = spec_routes();
    let unknown: Vec<_> = implemented()
        .iter()
        .filter(|route| !spec.contains(&key(route)))
        .map(Route::to_string)
        .collect();
    assert!(unknown.is_empty(), "not in the spec: {unknown:#?}");
}

#[test]
fn implemented_routes_are_unique_per_module() {
    for routes in modules() {
        let unique: BTreeSet<_> = routes.iter().map(key).collect();
        assert_eq!(
            unique.len(),
            routes.len(),
            "an Endpoint is declared twice in {routes:?}"
        );
    }
}

/// Every entry still names spec routes, and none of them is implemented after all
#[test]
fn out_of_scope_entries_match_the_spec() {
    let spec = spec_routes();
    let implemented: BTreeSet<_> = implemented().iter().map(key).collect();
    for (entry, _) in OUT_OF_SCOPE {
        assert!(
            spec.iter().any(|route| covers(entry, route)),
            "{entry} matches no spec route"
        );
        let both: Vec<_> = implemented
            .iter()
            .filter(|route| covers(entry, route))
            .collect();
        assert!(
            both.is_empty(),
            "{entry} is out of scope but implemented: {both:?}"
        );
    }
}

/// Not a check: `cargo test -p composure --features full --lib rest::spec -- --nocapture`
#[test]
fn unimplemented_routes_report() {
    let implemented: BTreeSet<_> = implemented().iter().map(key).collect();
    let (skipped, missing): (Vec<_>, Vec<_>) = spec_routes()
        .into_iter()
        .filter(|route| !implemented.contains(route))
        .partition(out_of_scope);
    println!(
        "{} spec routes not implemented ({} more out of scope):",
        missing.len(),
        skipped.len()
    );
    for (method, path) in missing {
        println!("  {method:6} {path}");
    }
}

#[test]
fn normalizes_parameter_names() {
    assert_eq!(normalize("/a/{x_id}/b/{y}"), "/a/{}/b/{}");
}
