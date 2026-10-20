use composure::Modal;

#[derive(Modal)]
#[modal(title = "Options")]
struct Free {
    #[select]
    free: String,
}

fn main() {}
