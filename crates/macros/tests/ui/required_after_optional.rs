/// Greet
#[composure::command]
fn greet(
    #[option("Greeting")] greeting: Option<String>,
    #[option("Name")] name: String,
) -> String {
    format!("{greeting:?} {name}")
}

fn main() {}
