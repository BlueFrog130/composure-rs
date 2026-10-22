use composure::{app, command::HandlerError, types::InteractionResponse};

fn render(error: &HandlerError) -> InteractionResponse {
    error.response()
}

/// A command
#[composure::command]
fn ping() -> &'static str {
    "pong"
}

#[app(handlers(ping), errors = render)]
struct Inner;

#[app(handlers(Inner), errors = render)]
struct Bot;

fn main() {}
