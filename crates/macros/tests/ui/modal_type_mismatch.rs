use composure::{Choice, Modal};

#[derive(Choice)]
enum Level {
    Low,
    High,
}

#[derive(Modal)]
#[modal(title = "Mismatch")]
struct Mismatch {
    #[checkbox]
    agreed: String,
    #[radio]
    levels: Vec<Level>,
    #[input]
    flag: bool,
}

fn main() {}
