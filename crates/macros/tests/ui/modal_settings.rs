use composure::Modal;

#[derive(Modal)]
#[modal(title = "Settings")]
struct TwoKinds {
    #[input]
    #[select(option("A", "a"))]
    both: String,
}

#[derive(Modal)]
#[modal(title = "Settings")]
struct BadSettings {
    #[radio(option("Only", "only"))]
    one: String,
}

#[derive(Modal)]
#[modal(title = "Settings")]
struct Unknown {
    #[checkbox(placeholder = "x")]
    agreed: bool,
}

fn main() {}
