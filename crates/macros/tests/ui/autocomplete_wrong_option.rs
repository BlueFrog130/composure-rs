use composure::{Choice, autocomplete, command, types::User};

#[autocomplete]
fn names(input: &str) -> Vec<String> {
    vec![input.to_string()]
}

#[derive(Choice)]
enum Size {
    Small,
    Large,
}

/// Fixed choices cannot autocomplete
#[command]
fn sized(#[option("Size", autocomplete = names)] size: Size) -> String {
    let _ = size;
    String::new()
}

/// Users cannot autocomplete
#[command]
fn who(#[option("Who", autocomplete = names)] user: &User) -> String {
    user.username.clone()
}

fn main() {}
