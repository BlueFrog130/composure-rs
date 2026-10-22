use composure::{command::View, rsx, view};

#[view]
fn Card(title: String) -> View {
    rsx! {
        <Container>
            <Button custom_id="x">"Loose"</Button>
            <Text>{title}</Text>
        </Container>
        <Row>"Text"</Row>
    }
}

fn main() {
    // The Props and their builder are still generated
    let _ = rsx! { <Card title="Hi"/> };
}
