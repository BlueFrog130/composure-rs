use composure::{app, command::EventHandler};

struct Events;

impl<S: Sync> EventHandler<S> for Events {}

/// A command
#[composure::command]
fn ping() -> &'static str {
    "pong"
}

#[app(handlers(ping), events = Events)]
struct Inner;

#[app(handlers(Inner), events = Events)]
struct Bot;

fn main() {}
