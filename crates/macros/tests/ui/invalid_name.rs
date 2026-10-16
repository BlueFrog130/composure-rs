/// Echo
#[composure::command(name = "Echo Text")]
fn echo() -> &'static str {
    "hi"
}

fn main() {}
