//! Checks every implemented Endpoint against Discord's OpenAPI spec (vendored at the commit in
//! `spec/PINNED`; bump with `scripts/update-spec.sh`) and reports what is not implemented yet.

use std::collections::BTreeSet;

use super::Route;

fn implemented() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.extend_from_slice(super::interaction::ROUTES);
    #[cfg(feature = "rest-commands")]
    routes.extend_from_slice(super::commands::ROUTES);
    #[cfg(feature = "rest-messages")]
    routes.extend_from_slice(super::messages::ROUTES);
    #[cfg(feature = "rest-users")]
    routes.extend_from_slice(super::users::ROUTES);
    #[cfg(feature = "rest-channels")]
    routes.extend_from_slice(super::channels::ROUTES);
    #[cfg(feature = "rest-guilds")]
    routes.extend_from_slice(super::guilds::ROUTES);
    #[cfg(feature = "rest-members")]
    routes.extend_from_slice(super::members::ROUTES);
    #[cfg(feature = "rest-roles")]
    routes.extend_from_slice(super::roles::ROUTES);
    routes
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
fn implemented_routes_are_unique() {
    let routes = implemented();
    let unique: BTreeSet<_> = routes.iter().map(key).collect();
    assert_eq!(unique.len(), routes.len(), "an Endpoint is declared twice");
}

/// Not a check: `cargo test -p composure --all-features unimplemented -- --nocapture`
#[test]
fn unimplemented_routes_report() {
    let implemented: BTreeSet<_> = implemented().iter().map(key).collect();
    let missing: Vec<_> = spec_routes()
        .into_iter()
        .filter(|route| !implemented.contains(route))
        .collect();
    println!("{} spec routes not implemented:", missing.len());
    for (method, path) in missing {
        println!("  {method:6} {path}");
    }
}

#[test]
fn normalizes_parameter_names() {
    assert_eq!(normalize("/a/{x_id}/b/{y}"), "/a/{}/b/{}");
}
