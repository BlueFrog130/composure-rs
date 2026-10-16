/// Add
#[composure::command]
fn add() -> &'static str {
    "added"
}

/// Inner
#[composure::group(subcommands(add))]
struct Inner;

/// Middle
#[composure::group(subcommands(Inner))]
struct Middle;

/// Outer
#[composure::group(subcommands(Middle))]
struct Outer;

fn main() {}
