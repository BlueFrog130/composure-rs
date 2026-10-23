//! The changelog, one release per page: `/changelog [version]`, `/compare <from> <to>` (the same
//! pages, limited to a range) and the "Explain release" message command.
//!
//! The [`Changelog`] View carries who opened it and the range in its `custom_id`s; the page
//! itself is re-read from the stored releases on every click. Only the user who opened a page
//! turns it; anyone else clicking gets their own ephemeral copy.

use composure::{
    Ctx, app, autocomplete, command,
    command::{ComponentCtx, Ephemeral, HandlerError, IntoResponse, Update, View},
    rsx,
    types::{Component, InteractionResponse, Message, SelectOption, Snowflake},
    view,
};

use crate::{
    Services,
    releases::{self, Release},
    ui,
};

/// Room for the notes: Discord allows 4000 characters of text per message, the rest is headings
const NOTES_LIMIT: usize = 3400;

/// Discord's limit on select options
const MAX_OPTIONS: usize = 25;

/// What a page shows, read from the releases; not carried in `custom_id`s
pub struct Page {
    release: Release,
    /// 1-based, within the range
    number: usize,
    total: usize,
    /// Versions offered by the jump menu, around this one
    versions: Vec<String>,
    /// `/compare`'s summary line
    heading: Option<String>,
}

/// One release of the changelog with buttons to the newer and older ones, a jump menu and a link
/// to the release. `owner`, `from`, `to` and the neighbours are its state.
#[view(prefix = "cl")]
pub fn Changelog(
    owner: Snowflake,
    from: Option<String>,
    to: Option<String>,
    newer: Option<String>,
    older: Option<String>,
    page: Page,
) -> View {
    let (notes, cut) = releases::excerpt(&releases::tidy(&page.release.notes), NOTES_LIMIT);
    let more = cut.then(|| format!("\n\n-# … [Read the rest on GitHub]({})", page.release.url));
    let options: Vec<SelectOption> = page
        .versions
        .iter()
        .map(|version| {
            SelectOption::new(
                format!("v{version}"),
                version.clone(),
                None,
                None,
                Some(*version == page.release.version),
            )
        })
        .collect();
    rsx! {
        <ui::Primary>
            {page.heading.map(|heading| rsx! { <Text>"-# " {heading}</Text> })}
            <Text>"## composure v" {page.release.version.as_str()}</Text>
            <Text>"-# Released " {page.release.date.as_str()} " · " {page.number} " of " {page.total}</Text>
            <Separator/>
            <Text>{notes} {more}</Text>
            <Separator/>
            <Row>
                <Button disabled=newer.is_none() on:click:newer={async move |ctx: ComponentCtx<impl Services>| {
                    turn(ctx, owner, from, to, newer).await
                }}>"◀ Newer"</Button>
                <Button disabled=older.is_none() on:click:older={async move |ctx: ComponentCtx<impl Services>| {
                    turn(ctx, owner, from, to, older).await
                }}>"Older ▶"</Button>
                <Button url=page.release.url.as_str()>"On GitHub"</Button>
            </Row>
            <Row>
                <StringSelect placeholder="Jump to a release" on:select:jump={async move |ctx: ComponentCtx<impl Services>, version: String| {
                    turn(ctx, owner, from, to, Some(version)).await
                }}>
                    {options}
                </StringSelect>
            </Row>
        </ui::Primary>
    }
}

/// Opens `version` for whoever clicked: in place for the page's owner, else as their own
/// ephemeral copy
async fn turn(
    ctx: ComponentCtx<'_, impl Services>,
    owner: Snowflake,
    from: Option<String>,
    to: Option<String>,
    version: Option<String>,
) -> Result<InteractionResponse, HandlerError> {
    let version = version.ok_or_else(|| HandlerError::user("No more releases that way."))?;
    let clicker = invoker(ctx.user().map(|user| user.id))?;
    let releases = releases::load(ctx.state()).await?;
    let page = build(&releases, clicker, Some(&version), from, to)?;
    if clicker == owner {
        Update(page).try_into_response()
    } else {
        Ephemeral(page).try_into_response()
    }
}

fn invoker(user: Option<Snowflake>) -> Result<Snowflake, HandlerError> {
    user.ok_or_else(|| HandlerError::new("interaction without a user"))
}

/// The page for `version` (default: the newest in range) within `from..=to` (default: every
/// release), owned by `owner`
pub fn build(
    releases: &[Release],
    owner: Snowflake,
    version: Option<&str>,
    from: Option<String>,
    to: Option<String>,
) -> Result<Changelog, HandlerError> {
    if releases.is_empty() {
        return Err(HandlerError::user("There are no releases yet."));
    }
    let find = |version: &str| {
        releases::position(releases, version).ok_or_else(|| {
            HandlerError::user(format!(
                "There is no release `{version}`; pick one from the suggestions."
            ))
        })
    };
    // Newest first, so the newer end has the lower index
    let (first, last) = match (&from, &to) {
        (Some(from), Some(to)) => {
            let (a, b) = (find(from)?, find(to)?);
            (a.min(b), a.max(b))
        }
        _ => (0, releases.len() - 1),
    };
    let index = match version {
        Some(version) => find(version)?,
        None => first,
    };
    if !(first..=last).contains(&index) {
        return Err(HandlerError::user(
            "That release is outside this comparison.",
        ));
    }
    let window_start = index
        .saturating_sub(MAX_OPTIONS / 2)
        .max(first)
        .min((last + 1).saturating_sub(MAX_OPTIONS).max(first));
    let versions = releases[window_start..=last]
        .iter()
        .take(MAX_OPTIONS)
        .map(|release| release.version.clone())
        .collect();
    let heading = from.is_some().then(|| {
        format!(
            "Changes from v{} to v{}: {} releases",
            releases[last].version,
            releases[first].version,
            last - first + 1
        )
    });
    Ok(Changelog {
        owner,
        newer: (index > first).then(|| releases[index - 1].version.clone()),
        older: (index < last).then(|| releases[index + 1].version.clone()),
        from,
        to,
        page: Page {
            release: releases[index].clone(),
            number: index - first + 1,
            total: last - first + 1,
            versions,
            heading,
        },
    })
}

/// Suggests released versions matching what was typed, newest first
#[autocomplete]
pub async fn versions(
    ctx: Ctx<impl Services>,
    input: &str,
) -> Result<Vec<(String, String)>, HandlerError> {
    let input = input.trim().trim_start_matches('v');
    Ok(releases::load(ctx.state())
        .await?
        .into_iter()
        .filter(|release| release.version.starts_with(input))
        .take(MAX_OPTIONS)
        .map(|release| {
            (
                format!("v{} ({})", release.version, release.date),
                release.version,
            )
        })
        .collect())
}

/// Read Composure's changelog, one release per page
#[command(
    contexts(Guild, BotDm, PrivateChannel),
    integration_types(GuildInstall, UserInstall)
)]
pub async fn changelog(
    ctx: Ctx<impl Services>,
    #[option("Release to open (default: the latest)", autocomplete = versions)] version: Option<
        &str,
    >,
) -> Result<Changelog, HandlerError> {
    let owner = invoker(ctx.user().map(|user| user.id))?;
    build(
        &releases::load(ctx.state()).await?,
        owner,
        version,
        None,
        None,
    )
}

/// See what changed between two releases
#[command(
    contexts(Guild, BotDm, PrivateChannel),
    integration_types(GuildInstall, UserInstall)
)]
pub async fn compare(
    ctx: Ctx<impl Services>,
    #[option("The older release", autocomplete = versions)] from: &str,
    #[option("The newer release (default: the latest)", autocomplete = versions)] to: Option<&str>,
) -> Result<Changelog, HandlerError> {
    let owner = invoker(ctx.user().map(|user| user.id))?;
    let releases = releases::load(ctx.state()).await?;
    let to = match to {
        Some(to) => to.to_string(),
        None => releases
            .first()
            .map(|release| release.version.clone())
            .unwrap_or_default(),
    };
    build(&releases, owner, None, Some(from.to_string()), Some(to))
}

/// Opens the changelog at the version a message mentions, e.g. a release announcement
#[command(
    message,
    name = "Explain release",
    contexts(Guild, BotDm, PrivateChannel),
    integration_types(GuildInstall, UserInstall)
)]
pub async fn explain_release(
    ctx: Ctx<impl Services>,
    target: &Message,
) -> Result<Ephemeral<Changelog>, HandlerError> {
    let owner = invoker(ctx.user().map(|user| user.id))?;
    let texts = std::iter::once(target.content.as_str())
        .chain(target.embeds.iter().flat_map(|embed| {
            [embed.title.as_deref(), embed.description.as_deref()]
                .into_iter()
                .flatten()
        }))
        .chain(text_displays(
            target.components.as_deref().unwrap_or_default(),
        ));
    let version = texts
        .filter_map(releases::find_version)
        .next()
        .ok_or_else(|| {
            HandlerError::user("That message doesn't mention a composure version like `0.1.2`.")
        })?;
    let releases = releases::load(ctx.state()).await?;
    Ok(Ephemeral(build(
        &releases,
        owner,
        Some(version),
        None,
        None,
    )?))
}

/// Every Text Display's content in `components`, depth first: announcements are Components V2,
/// with no content or embeds
fn text_displays(components: &[Component]) -> Vec<&str> {
    let mut texts = Vec::new();
    for component in components {
        match component {
            Component::TextDisplay(text) => texts.push(text.content.as_str()),
            Component::Container(container) => texts.extend(text_displays(&container.components)),
            Component::Section(section) => texts.extend(text_displays(&section.components)),
            _ => {}
        }
    }
    texts
}

/// The changelog Commands and their View, listed in [`ComposureBot`](crate::ComposureBot)
#[app(handlers(Changelog, changelog, compare, explain_release))]
pub struct Changelogs;
