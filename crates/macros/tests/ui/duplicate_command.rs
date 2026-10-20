use composure::app;

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

#[app(handlers(a, other::b))]
struct Bot;

fn main() {}
