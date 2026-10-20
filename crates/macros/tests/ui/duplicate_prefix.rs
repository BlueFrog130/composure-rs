use composure::{command, app, component};

/// A command
#[command]
fn ping() -> &'static str {
    "pong"
}

#[component("same")]
fn first() -> String {
    String::new()
}

#[component("same")]
fn second() -> String {
    String::new()
}

#[app(handlers(ping, first, second))]
pub struct Bot;

fn main() {}
