use composure::models::Message;

#[composure::command(user, name = "Inspect")]
fn inspect(target: Message) -> String {
    target.content
}

fn main() {}
