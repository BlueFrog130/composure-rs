//! Webhook Events: every install is logged to the maintainers' log channel, and whoever adds
//! the bot to a server gets a DM on setting up announcements.

use composure::{
    command::{EventCtx, EventHandler, HandlerError},
    rsx,
    types::{ApplicationAuthorized, ApplicationIntegrationType},
};

use crate::{Services, ui};

const GUIDE: &str = "https://bluefrog130.github.io/composure-rs/";

/// `APPLICATION_AUTHORIZED` handling, registered with `#[app(.., events = Events)]`
pub struct Events;

impl<S: Services> EventHandler<S> for Events {
    async fn application_authorized(
        &self,
        ctx: EventCtx<'_, S>,
        event: ApplicationAuthorized,
    ) -> Result<(), HandlerError> {
        let bot = ctx.bot()?;
        let user = &event.user;
        let by = format!("<@{}> ({})", user.id, user.username);
        let (log, guild) = match (&event.integration_type, event.guild) {
            (Some(ApplicationIntegrationType::GuildInstall), Some(guild)) => (
                rsx! {
                    <ui::Secondary title="Added to a server">
                        "**" {guild.name.as_str()} "** (`" {guild.id} "`) by " {by}
                    </ui::Secondary>
                },
                Some(guild),
            ),
            (Some(ApplicationIntegrationType::UserInstall), _) => (
                rsx! {
                    <ui::Secondary title="Added to a user's apps">{by}</ui::Secondary>
                },
                None,
            ),
            _ => return Ok(()),
        };
        ui::send(&bot, ctx.state().data().log_channel, log).await?;
        tracing::info!(user_id = %user.id, "logged install");

        let Some(guild) = guild else {
            return Ok(());
        };
        let welcome = rsx! {
            <ui::Primary title="Thanks for adding Composure!">
                "In **" {guild.name.as_str()} "**, run `/notify setup` to get an announcement "
                "whenever a new version is released, or `/changelog` to read what changed."
                <Row>
                    <Button url=GUIDE>"Read the guide"</Button>
                </Row>
            </ui::Primary>
        };
        // Best effort: the installer may not accept DMs
        let sent = async {
            let dm = bot.create_dm(user.id).await?.model()?;
            ui::send(&bot, dm.id, welcome).await
        }
        .await;
        if let Err(error) = sent {
            tracing::debug!(%error, "could not DM the installer");
        }
        Ok(())
    }
}
