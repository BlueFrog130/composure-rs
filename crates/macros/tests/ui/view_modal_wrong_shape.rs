use composure::{
    command::{Ephemeral, ModalView},
    rsx, view,
};

#[view]
fn Form() -> ModalView {
    rsx! {
        <Modal title="Form" on:submit:send=|_ctx, agree: bool| Ephemeral(format!("{agree}"))>
            <Label text="Agree?"><TextInput name="agree"/></Label>
        </Modal>
    }
}

fn main() {}
