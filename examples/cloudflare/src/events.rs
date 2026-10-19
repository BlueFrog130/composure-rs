//! Webhook Events, registered with `commands! { ..; events = Events; }` and served on `/events`.

use composure::{
    command::{EventCtx, EventHandler, HandlerError, Platform},
    types::{ApplicationAuthorized, ApplicationIntegrationType},
};

/// Welcomes users who install the app to their account
pub struct Events;

impl<S: Platform> EventHandler<S> for Events {
    async fn application_authorized(
        &self,
        ctx: EventCtx<'_, S>,
        event: ApplicationAuthorized,
    ) -> Result<(), HandlerError> {
        if event.integration_type != Some(ApplicationIntegrationType::UserInstall) {
            return Ok(());
        }
        // Logged inside Composure's `event` span, so it carries `event_type`
        tracing::info!(user_id = %event.user.id, "installed by a user; sending a welcome DM");
        // Handled after Discord got its 204, so this can take its time
        let bot = ctx.bot()?;
        let dm = bot.create_dm(event.user.id).await?.model()?;
        bot.send_message(dm.id, "Thanks for installing! Try `/hello` anywhere.")
            .await?;
        Ok(())
    }
}
