use composure::{command::View, rsx, view};

#[view]
fn Card(title: String, likes: u32) -> View {
    rsx! { <Text>{title} {likes}</Text> }
}

fn main() {
    let _ = rsx! { <Card title="x"/> };
}
