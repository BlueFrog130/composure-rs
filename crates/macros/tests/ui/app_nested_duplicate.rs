use composure::app;

/// A command
#[composure::command]
fn ping() -> &'static str {
    "pong"
}

#[app(handlers(ping))]
struct Inner;

#[app(handlers(ping, Inner))]
struct Bot;

fn main() {}
