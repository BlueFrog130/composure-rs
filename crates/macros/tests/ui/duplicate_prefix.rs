use composure::{command, commands, component};

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

commands! {
    pub struct Bot = [ping];
    components = [first, second];
}

fn main() {}
