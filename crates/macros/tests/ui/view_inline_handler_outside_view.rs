use composure::{command::Ephemeral, rsx};

fn main() {
    let _ = rsx! { <Row><Button on:click:hi=|_| Ephemeral("hi")>"Hi"</Button></Row> };
}
