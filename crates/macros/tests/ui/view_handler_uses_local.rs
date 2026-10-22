use composure::{
    command::{Ephemeral, View},
    rsx, view,
};

#[view]
fn Total(count: u32) -> View {
    let total = count * 2;
    rsx! {
        <Row>
            <Button on:click:show=move |_| Ephemeral(format!("{total}"))>"Show"</Button>
        </Row>
    }
}

#[view]
fn List(items: Vec<String>) -> View {
    rsx! {
        {items.iter().map(|item| rsx! {
            <Row>
                <Button on:click:pick=move |_| Ephemeral(item.clone())>"Pick"</Button>
            </Row>
        })}
    }
}

#[view]
fn Title(title: String) -> View {
    let title = title.trim().to_string();
    rsx! {
        <Text>{title.as_str()}</Text>
        <Row>
            <Button on:click:echo=move |_| Ephemeral(title)>"Echo"</Button>
        </Row>
    }
}

fn main() {}
