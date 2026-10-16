use composure::models::User;

#[composure::command(user, name = "Inspect")]
fn inspect(target: User) -> String {
    target.username
}

/// Admin
#[composure::group(subcommands(inspect))]
struct Admin;

fn main() {}
