use composure::component;

#[component("a:b")]
fn bad_prefix() -> String {
    String::new()
}

fn main() {}
