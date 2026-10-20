use composure::rsx;

fn main() {
    let _ = rsx! { <Container><Button custom_id="x">"Loose"</Button></Container> };
}
