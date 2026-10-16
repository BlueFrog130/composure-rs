use composure::{Modal, modal, command::ModalCtx};

#[derive(Modal)]
struct NoTitle {
    name: String,
}

#[modal("m")]
fn no_form(_ctx: ModalCtx) -> String {
    String::new()
}

fn main() {}
