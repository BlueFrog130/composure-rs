use composure::{Ctx, component};

#[component("x")]
fn wrong_ctx(_ctx: Ctx) -> String {
    String::new()
}

fn main() {}
