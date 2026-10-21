use composure::{app, command::HandlerError};

/// Not a renderer: returns text instead of a response
fn render(error: &HandlerError) -> String {
    error.to_string()
}

/// A command
#[composure::command]
fn ping() -> &'static str {
    "pong"
}

#[app(handlers(ping), errors = render)]
struct Bot;

fn main() {}
