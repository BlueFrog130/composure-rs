use composure::{command::View, rsx, view};

#[view]
fn Card(title: &str) -> View {
    rsx! { <Text>{title}</Text> }
}

fn main() {}
