/// Greet
#[composure::command]
fn greet(
    #[option(desc = "Greeting")] greeting: Option<String>,
    #[option(desc = "Name")] name: String,
) -> String {
    format!("{greeting:?} {name}")
}

fn main() {}
