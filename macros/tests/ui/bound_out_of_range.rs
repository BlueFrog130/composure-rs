/// Count
#[composure::command]
fn count(#[option(desc = "How many", max = 300)] n: u8) -> String {
    n.to_string()
}

fn main() {}
