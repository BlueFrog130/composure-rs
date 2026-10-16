use composure::commands;

/// A
#[composure::command]
fn a() -> &'static str {
    "a"
}

mod other {
    /// Also a
    #[composure::command(name = "a")]
    pub fn b() -> &'static str {
        "b"
    }
}

commands! {
    struct Bot = [a, other::b];
}

fn main() {}
