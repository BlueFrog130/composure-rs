//! Workers KV: one private note per user in the `NOTES` namespace (`kv_namespaces` in
//! wrangler.toml), expiring after 30 days.
//!
//! KV calls are JS promises, so their futures are not `Send` while Composure's handler futures
//! must be (ADR 0012). Each helper is `#[worker::send]`, which wraps its body in a `SendFuture`
//! (sound: a Worker runs on one thread). The helpers take the `Worker` State rather than an
//! `&Env`: `Worker` is `Sync`, `Env` is not, and the argument is held across the `.await`.

use composure::{
    Ctx, command,
    command::{Ephemeral, HandlerError, View},
    group, rsx,
};
use composure_adapter_cloudflare::Worker;
use composure_example_bot::Config;

/// The KV binding in wrangler.toml
const NOTES: &str = "NOTES";

const TTL_DAYS: u64 = 30;

/// Keep a private note in Workers KV
#[group(subcommands(save_note, show_note, forget_note))]
pub struct Note;

/// Save a note only you can read (replaces the old one)
#[command(name = "save")]
pub async fn save_note(
    ctx: Ctx<Worker<Config>>,
    #[option("The note", max_len = 1000)] text: &str,
) -> Result<Ephemeral<String>, HandlerError> {
    put(ctx.state(), &key(&ctx)?, text)
        .await
        .map_err(HandlerError::new)?;
    Ok(Ephemeral(format!(
        "Saved. `/note show` brings it back for {TTL_DAYS} days."
    )))
}

/// Show your note
#[command(name = "show")]
pub async fn show_note(ctx: Ctx<Worker<Config>>) -> Result<Ephemeral<View>, HandlerError> {
    let note = get(ctx.state(), &key(&ctx)?)
        .await
        .map_err(HandlerError::new)?;
    Ok(Ephemeral(match note {
        Some(note) => rsx! {
            <Container>
                "##  Your note"
                <Separator />
                ">>> " {note}
            </Container>
        },
        None => rsx! { <Container>"You have no note. Save one with `/note save`."</Container> },
    }))
}

/// Delete your note
#[command(name = "forget")]
pub async fn forget_note(
    ctx: Ctx<Worker<Config>>,
) -> Result<Ephemeral<&'static str>, HandlerError> {
    delete(ctx.state(), &key(&ctx)?)
        .await
        .map_err(HandlerError::new)?;
    Ok(Ephemeral("Forgotten."))
}

/// Notes are per user, in guilds and DMs alike
fn key(ctx: &Ctx<Worker<Config>>) -> Result<String, HandlerError> {
    let user = ctx
        .user()
        .ok_or_else(|| HandlerError::new("interaction without a user"))?;
    Ok(format!("note:{}", user.id))
}

#[worker::send]
async fn put(worker: &Worker<Config>, key: &str, text: &str) -> worker::Result<()> {
    worker
        .env()
        .kv(NOTES)?
        .put(key, text)?
        .expiration_ttl(TTL_DAYS * 24 * 60 * 60)
        .execute()
        .await?;
    Ok(())
}

#[worker::send]
async fn get(worker: &Worker<Config>, key: &str) -> worker::Result<Option<String>> {
    Ok(worker.env().kv(NOTES)?.get(key).text().await?)
}

#[worker::send]
async fn delete(worker: &Worker<Config>, key: &str) -> worker::Result<()> {
    Ok(worker.env().kv(NOTES)?.delete(key).await?)
}
