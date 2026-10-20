use composure::{
    command::{Ephemeral, View},
    app, rsx, view,
};

#[view]
fn Row1(id: u32) -> View {
    rsx! { <Row><Button on:click:pick=move |_| Ephemeral(format!("{id}"))>"Pick"</Button></Row> }
}

#[view]
fn List() -> View {
    rsx! { <Container><Row1 id=1/></Container> }
}

/// A command
#[composure::command]
fn list() -> List {
    List {}
}

#[app(handlers(list, List))]
pub struct Bot;

fn main() {}
