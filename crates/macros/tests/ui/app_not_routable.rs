use composure::app;

/// A command
#[composure::command]
fn ping() -> &'static str {
    "pong"
}

struct Plain;

#[app(handlers(ping, Plain))]
struct Bot;

fn main() {}
