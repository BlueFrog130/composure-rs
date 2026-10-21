//! Release announcements. The cron trigger posts each new release in the home server's
//! announcement channel and publishes it; servers get it through Discord's channel following,
//! set up by `/notify setup`. Nothing is stored per server: a server's subscriptions are its
//! channel follower webhooks whose source is that announcement channel.

use composure::{
    Ctx, command,
    command::{Ephemeral, HandlerError, View},
    group,
    rest::{BotClient, Error as RestError, Transport},
    rsx,
    types::{PartialChannel, Snowflake, Webhook},
    view,
};

use crate::{
    Config, Services, Store,
    releases::{self, Release},
    ui,
};

/// The last version announced
pub const ANNOUNCED_KEY: &str = "announced";

/// Room for the notes within Discord's 4000 characters of text per message
const NOTES_LIMIT: usize = 3500;

/// Webhook type of a channel follower
const CHANNEL_FOLLOWER: u8 = 2;

/// Get Composure release announcements in this server
#[group(
    subcommands(notify_setup, notify_remove, notify_status, notify_preview),
    contexts(Guild),
    integration_types(GuildInstall),
    permissions = Permissions::ManageGuild
)]
pub struct Notify;

/// Announce new Composure releases in a channel
#[command(name = "setup")]
pub async fn notify_setup(
    ctx: Ctx<impl Services>,
    #[option("Where announcements go", channel_types(GuildText, GuildAnnouncement))]
    channel: &PartialChannel,
) -> Result<Ephemeral<View>, HandlerError> {
    let guild = guild(&ctx)?;
    let config = ctx.state().data();
    if channel.id == config.announce_channel {
        return Err(HandlerError::user(
            "That is the announcement channel itself.",
        ));
    }
    let bot = ctx.bot()?;
    let current = followers(&bot, guild, config).await?;
    if current
        .iter()
        .any(|webhook| webhook.channel_id == Some(channel.id))
    {
        return Ok(Ephemeral(rsx! {
            <ui::Secondary>"<#" {channel.id} "> already gets release announcements."</ui::Secondary>
        }));
    }
    let reason = format!(
        "/notify setup by {}",
        ctx.user().map_or("someone", |user| user.username.as_str())
    );
    bot.follow_announcement_channel(config.announce_channel, channel.id, Some(&reason))
        .await
        .map_err(|error| permission_error(error, channel.id))?;
    Ok(Ephemeral(rsx! {
        <ui::Success title="Announcements on">
            "New Composure releases will be announced in <#" {channel.id} ">."
            <Text>"-# `/notify preview` shows what they look like; `/notify remove` stops them."</Text>
        </ui::Success>
    }))
}

/// Stop release announcements
#[command(name = "remove")]
pub async fn notify_remove(
    ctx: Ctx<impl Services>,
    #[option(
        "Only this channel (default: every channel)",
        channel_types(GuildText, GuildAnnouncement)
    )]
    channel: Option<&PartialChannel>,
) -> Result<Ephemeral<View>, HandlerError> {
    let guild = guild(&ctx)?;
    let bot = ctx.bot()?;
    let removed: Vec<Webhook> = followers(&bot, guild, ctx.state().data())
        .await?
        .into_iter()
        .filter(|webhook| channel.is_none_or(|channel| webhook.channel_id == Some(channel.id)))
        .collect();
    if removed.is_empty() {
        return Ok(Ephemeral(rsx! {
            <ui::Secondary>"There were no release announcements to stop."</ui::Secondary>
        }));
    }
    for webhook in &removed {
        bot.delete_webhook(webhook.id, Some("/notify remove"))
            .await
            .map_err(|error| permission_error(error, webhook.channel_id.unwrap_or(guild)))?;
    }
    Ok(Ephemeral(rsx! {
        <ui::Success title="Announcements off">
            "Stopped release announcements in " {mentions(&removed)} "."
        </ui::Success>
    }))
}

/// Where release announcements go in this server
#[command(name = "status")]
pub async fn notify_status(ctx: Ctx<impl Services>) -> Result<Ephemeral<View>, HandlerError> {
    let guild = guild(&ctx)?;
    let current = followers(&ctx.bot()?, guild, ctx.state().data()).await?;
    Ok(Ephemeral(if current.is_empty() {
        rsx! {
            <ui::Secondary title="Announcements off">
                "Turn them on with `/notify setup`."
            </ui::Secondary>
        }
    } else {
        rsx! {
            <ui::Secondary title="Announcements on">
                "Release announcements go to " {mentions(&current)} "."
            </ui::Secondary>
        }
    }))
}

/// Show the latest release's announcement, only to you
#[command(name = "preview")]
pub async fn notify_preview(
    ctx: Ctx<impl Services>,
) -> Result<Ephemeral<Announcement>, HandlerError> {
    let release = releases::load(ctx.state())
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| HandlerError::user("There are no releases yet."))?;
    Ok(Ephemeral(Announcement { release }))
}

fn guild(ctx: &Ctx<impl Services>) -> Result<Snowflake, HandlerError> {
    ctx.guild_id()
        .copied()
        .ok_or_else(|| HandlerError::user("Use this in a server."))
}

/// This server's webhooks following the announcement channel
async fn followers<T: Transport>(
    bot: &BotClient<T>,
    guild: Snowflake,
    config: &Config,
) -> Result<Vec<Webhook>, HandlerError> {
    let webhooks = bot
        .get_guild_webhooks(guild)
        .await
        .map_err(|error| permission_error(error, guild))?
        .model()?;
    Ok(webhooks
        .into_iter()
        .filter(|webhook| {
            webhook.kind == CHANNEL_FOLLOWER
                && webhook
                    .source_channel
                    .as_ref()
                    .is_some_and(|source| source.id == config.announce_channel)
        })
        .collect())
}

/// A missing permission as a message the user can act on; other failures as they are
fn permission_error(error: RestError, place: Snowflake) -> HandlerError {
    match &error {
        RestError::Api(api) if api.status == 403 => HandlerError::user(format!(
            "I need the **Manage Webhooks** permission there (<#{place}>) to manage announcements. Grant it to my role, or add me again with `/invite`."
        )),
        _ => error.into(),
    }
}

fn mentions(webhooks: &[Webhook]) -> String {
    webhooks
        .iter()
        .filter_map(|webhook| webhook.channel_id)
        .map(|channel| format!("<#{channel}>"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The announcement of a release: its notes and links in a Components V2 container. Only link
/// buttons, so it works wherever it is crossposted.
#[view]
pub fn Announcement(release: Release) -> View {
    let (notes, cut) = releases::excerpt(&releases::tidy(&release.notes), NOTES_LIMIT);
    let more = cut.then(|| format!("\n\n-# … [Read the rest on GitHub]({})", release.url));
    let version = release.version.as_str();
    rsx! {
        <ui::Primary>
            <Text>"## composure v" {version} " is out"</Text>
            <Text>"-# Released " {release.date.as_str()}</Text>
            <Separator/>
            <Text>{notes} {more}</Text>
            <Separator/>
            <Row>
                <Button url=release.url.as_str()>"Release notes"</Button>
                <Button url={format!("https://crates.io/crates/composure/{version}")}>"crates.io"</Button>
                <Button url={format!("https://docs.rs/composure/{version}")}>"docs.rs"</Button>
            </Row>
        </ui::Primary>
    }
}

/// The cron trigger's work: refreshes the stored releases, then posts and publishes every
/// release newer than the last one announced, oldest first. The first run (nothing announced
/// yet) announces every release, so the announcement channel starts with the full history.
/// Returns the versions announced.
pub async fn announce_new<T: Transport>(
    bot: &BotClient<T>,
    transport: &impl Transport,
    store: &impl Store,
    config: &Config,
) -> Result<Vec<String>, HandlerError> {
    let releases = releases::refresh(transport, store, config).await?;
    tracing::info!(?releases);
    let announced = store
        .get(ANNOUNCED_KEY)
        .await?
        .and_then(|bytes| String::from_utf8(bytes).ok());
    tracing::info!(?announced);
    let index = match announced {
        // First run: everything, oldest first
        None => releases.len(),
        Some(version) => match releases::position(&releases, &version) {
            Some(index) => index,
            // That release was deleted: catch up to the latest without reposting history
            None => {
                if let Some(latest) = releases.first() {
                    store
                        .put(ANNOUNCED_KEY, latest.version.clone().into_bytes(), None)
                        .await?;
                }
                return Ok(Vec::new());
            }
        },
    };
    let mut done = Vec::new();
    for release in releases[..index].iter().rev() {
        tracing::info!(?release.version);
        let announcement = Announcement {
            release: release.clone(),
        };
        tracing::info!(channel = ?config.announce_channel, "announcement channel");
        let message = ui::send(bot, config.announce_channel, announcement.render()).await?;
        // Already posted: a failed publish is logged, not retried, so it is never posted twice
        if let Err(error) = bot
            .crosspost_message(config.announce_channel, message.id)
            .await
        {
            tracing::warn!(%error, version = %release.version, "could not publish the announcement");
        }
        store
            .put(ANNOUNCED_KEY, release.version.clone().into_bytes(), None)
            .await?;
        tracing::info!(version = %release.version, "announced release");
        done.push(release.version.clone());
    }
    Ok(done)
}
