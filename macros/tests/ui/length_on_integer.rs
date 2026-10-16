/// Count
#[composure::command]
fn count(#[option(desc = "How many", max_len = 3)] n: i64) -> String {
    n.to_string()
}

fn main() {}
