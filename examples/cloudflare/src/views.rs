//! Views: layouts written as `rsx!` markup and rendered from Props, with inline handlers for their
//! buttons and modals (`#[view]`). A handler's state is the Props it names, carried in the
//! `custom_id`; returning new Props re-renders the message.

use std::time::Duration;

use composure::{
    Choice,
    command::{ComponentCtx, Ephemeral, ModalView, Platform, View},
    rsx, view,
};

/// A Components V2 card (accent color from the theme): like it, or review it in a modal
#[view]
pub fn Card(likes: u32) -> View {
    rsx! {
        <Container>
            <Section>
                <Text>"## Components V2"</Text>
                <Text>"Sections, separators, galleries and files in a container."</Text>
                <Button url="https://discord.com/developers/docs/components/reference">"Docs"</Button>
            </Section>
            <Separator/>
            <Text>"-# " {likes} " likes"</Text>
            <Row>
                <Button primary on:click:like=move |_| Card { likes: likes + 1 }>"Like"</Button>
                <Button on:click:review=move |_| Review { likes }>"Review"</Button>
            </Row>
        </Container>
    }
}

#[derive(Choice, Debug)]
pub enum Stars {
    #[choice(name = "★")]
    One,
    #[choice(name = "★★")]
    Two,
    #[choice(name = "★★★")]
    Three,
}

/// A modal View: its inputs come back as the submit handler's typed parameters, matched by name
#[view]
pub fn Review(likes: u32) -> ModalView {
    rsx! {
        <Modal title="Review the card" on:submit:send={move |_ctx, stars: Stars, comment: Option<String>| {
            let comment = comment.map(|c| format!(": {c}")).unwrap_or_default();
            Ephemeral(format!("{} for a card with {likes} likes{comment}", stars.name()))
        }}>
            <Label text="Stars">
                <RadioGroup name="stars" options=Stars/>
            </Label>
            <Label text="Comment" description="Optional">
                <TextInput name="comment" paragraph max_len=200/>
            </Label>
        </Modal>
    }
}

/// Re-rolls from Background Work: the handler needs the Platform, so it says so in its `ctx` type
#[view]
pub fn Dice(face: u32, rolls: u32) -> View {
    rsx! {
        <Text>"Rolled a " {face} " (" {rolls} " rolls)"</Text>
        <Row>
            <Button primary on:click:roll={move |ctx: ComponentCtx<impl Platform>| {
                ctx.defer_update(async move |ctx| {
                    ctx.sleep(Duration::from_secs(1)).await;
                    // No randomness source needed for a demo: the interaction id is unique enough
                    let rolled = ctx.interaction().common.id.to_u64() % 6 + 1;
                    Dice { face: rolled as u32, rolls: rolls + 1 }
                })
            }}>"Roll"</Button>
        </Row>
    }
}

/// A Components V2 card, built from markup
#[composure::command]
pub fn card() -> Card {
    Card { likes: 0 }
}

/// Roll a die that re-rolls in place
#[composure::command]
pub fn dice() -> Dice {
    Dice { face: 1, rolls: 0 }
}
