/// Hi
#[composure::command]
fn hi(#[option("Name")] name: String, ctx: composure::Ctx) -> String {
    let _ = ctx;
    name
}

fn main() {}
