use composure::{
    command::{Ephemeral, ModalView},
    rsx, view,
};

#[view]
fn Form() -> ModalView {
    rsx! {
        <Modal title="Form" on:submit:send={|_ctx, name: String, age: String| Ephemeral(format!("{name} {age}"))}>
            <Label text="Name"><TextInput name="name"/></Label>
        </Modal>
    }
}

fn main() {}
