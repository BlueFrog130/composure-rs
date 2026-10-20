use composure::app;

/// A command
#[composure::command]
fn ping() -> &'static str {
    "pong"
}

#[app(handlers(ping), colors = 1)]
struct Unknown;

#[app(handlers(ping), handlers(ping))]
struct Twice;

#[app(handlers(ping))]
struct Fields(u32);

fn main() {}
