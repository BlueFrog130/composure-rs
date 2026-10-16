use composure::autocomplete;

#[autocomplete]
fn no_input() -> Vec<String> {
    Vec::new()
}

#[autocomplete]
fn owned(input: String) -> Vec<String> {
    vec![input]
}

fn main() {}
