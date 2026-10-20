/// Kick
#[composure::command(scope = "dev")]
fn kick() -> &'static str {
    "kicked"
}

/// Admin
#[composure::group(subcommands(kick))]
struct Admin;

fn main() {}
