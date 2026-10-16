use composure::{Ctx, commands};

/// A
#[composure::command]
fn a(ctx: Ctx<u32>) -> String {
    ctx.state().to_string()
}

/// B
#[composure::command]
fn b(ctx: Ctx<String>) -> String {
    ctx.state().clone()
}

commands! {
    struct Bot = [a, b];
}

fn main() {
    let _ = futures::executor::block_on(composure::command::dispatch(
        serde_json::from_str("{}").unwrap(),
        &Bot,
        &0u32,
    ));
}
