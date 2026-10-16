/// Count
#[composure::command]
fn count(#[option("How many", min = 1.5)] n: u8) -> String {
    n.to_string()
}

fn main() {}
