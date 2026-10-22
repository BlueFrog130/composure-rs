//! `/docs <topic>`: the guide's chapters and headings (indexed from `book/` by `build.rs`) and
//! the main API items, suggested while typing.

use composure::{
    autocomplete, command,
    command::{HandlerError, View},
    rsx,
};

use crate::{build_info::GUIDE, ui};

/// A page or section to link to
#[derive(Debug)]
pub struct Entry {
    /// `Views › Inline handlers`
    pub title: &'static str,
    /// Where it is, with its anchor
    pub url: &'static str,
    /// Its first paragraph, shortened
    pub summary: &'static str,
}

macro_rules! api {
    ($($title:literal, $path:literal, $summary:literal;)*) => {
        &[$(Entry { title: $title, url: concat!("https://docs.rs/", $path), summary: $summary },)*]
    };
}

/// The items most code starts from, on docs.rs
static API: &[Entry] = api! {
    "#[command]", "composure/latest/composure/attr.command.html", "Turns a function into a slash or context-menu Command: Options from its parameters, the description from its doc comment, the response from its return value.";
    "#[group]", "composure/latest/composure/attr.group.html", "A Command made of subcommands (and subcommand groups), routed to each one's function.";
    "#[app]", "composure/latest/composure/attr.app.html", "Collects Commands, handlers, Views and nested Apps into one handler, checked at compile time for duplicates and custom_id collisions.";
    "#[view]", "composure/latest/composure/attr.view.html", "A Components V2 message or modal as a function of Props, with inline handlers for its buttons, selects and submits.";
    "rsx!", "composure/latest/composure/macro.rsx.html", "JSX-like markup for Components V2 messages and modals.";
    "#[component]", "composure/latest/composure/attr.component.html", "A button or select handler routed by its custom_id prefix, with its state encoded in the custom_id.";
    "#[modal]", "composure/latest/composure/attr.modal.html", "A modal submit handler receiving a typed #[derive(Modal)] form.";
    "#[autocomplete]", "composure/latest/composure/attr.autocomplete.html", "Suggestions for an Option while the user types.";
    "#[derive(Choice)]", "composure/latest/composure/derive.Choice.html", "An enum as an Option's fixed choices, or a select's options.";
    "#[derive(Modal)]", "composure/latest/composure/derive.Modal.html", "A struct as a modal form: each field a labeled input.";
    "Ctx", "composure/latest/composure/command/struct.Ctx.html", "What a handler gets: the Interaction, the user, the platform's State, and REST and defer helpers.";
    "HandlerError", "composure/latest/composure/command/struct.HandlerError.html", "A handler's failure: logged, and answered ephemerally; HandlerError::user shows its message.";
    "ErrorRenderer", "composure/latest/composure/command/type.ErrorRenderer.html", "How an App answers handler errors: #[app(errors = render)] instead of the default ephemeral text.";
    "Platform", "composure/latest/composure/command/trait.Platform.html", "What an Adapter's State provides for REST and Background Work: a Transport, the bot token, spawn.";
    "BotClient", "composure/latest/composure/rest/struct.BotClient.html", "Discord's REST API with the bot token, one feature per resource.";
    "CloudflareInteractionBot", "composure_adapter_cloudflare/latest/composure_adapter_cloudflare/struct.CloudflareInteractionBot.html", "Serves a bot from a Cloudflare Worker: signature checks, dispatch, Background Work with wait_until.";
    "TokioInteractionBot", "composure_adapter_tokio/latest/composure_adapter_tokio/struct.TokioInteractionBot.html", "Serves a bot from a hyper server on tokio.";
    "composure_sync", "composure_sync/latest/composure_sync/", "Makes Discord's registered commands match the App: plan, confirm, apply.";
};

/// Every searchable entry: guide first, then the API
fn entries() -> impl Iterator<Item = (usize, &'static Entry)> {
    GUIDE.iter().chain(API).enumerate()
}

/// Entries matching `query`, best first: title prefix, word prefix, title, then summary matches
fn search(query: &str) -> Vec<(usize, &'static Entry)> {
    let query = query.trim().to_lowercase();
    let mut hits: Vec<(u8, usize, &Entry)> = entries()
        .filter_map(|(i, entry)| {
            let title = entry.title.to_lowercase();
            let leaf = title.rsplit(" › ").next().unwrap_or(&title);
            let rank = if query.is_empty() {
                // Chapters and API items, not every heading
                if entry.title.contains(" › ") {
                    return None;
                } else {
                    0
                }
            } else if leaf.starts_with(&query) || title.starts_with(&query) {
                0
            } else if title
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| word.starts_with(&query))
            {
                1
            } else if title.contains(&query) {
                2
            } else if entry.summary.to_lowercase().contains(&query) {
                3
            } else {
                return None;
            };
            Some((rank, i, entry))
        })
        .collect();
    hits.sort_by_key(|&(rank, i, entry)| (rank, entry.title.len(), i));
    hits.into_iter().map(|(_, i, entry)| (i, entry)).collect()
}

/// Suggests guide sections and API items; the value is the entry's index
#[autocomplete]
pub fn topics(input: &str) -> Vec<(String, String)> {
    search(input)
        .into_iter()
        .take(25)
        .map(|(i, entry)| (entry.title.to_string(), i.to_string()))
        .collect()
}

/// Search Composure's guide and API docs
#[command(
    contexts(Guild, BotDm, PrivateChannel),
    integration_types(GuildInstall, UserInstall)
)]
pub fn docs(
    #[option("What to look for", autocomplete = topics, max_len = 100)] topic: &str,
) -> Result<View, HandlerError> {
    // A picked suggestion is an index; typed text is searched
    let entry = topic
        .parse::<usize>()
        .ok()
        .and_then(|i| entries().nth(i))
        .or_else(|| search(topic).into_iter().next())
        .map(|(_, entry)| entry)
        .ok_or_else(|| HandlerError::user(format!("Nothing in the docs matches `{topic}`.")))?;
    let source = if entry.url.starts_with("https://docs.rs/") {
        "API docs"
    } else {
        "Guide"
    };
    Ok(rsx! {
        <ui::Primary>
            <Section>
                <Text>"### " {entry.title}</Text>
                <Text>{entry.summary}</Text>
                <Button url=entry.url>"Open"</Button>
            </Section>
            <Text>"-# " {source} " · " {entry.url}</Text>
        </ui::Primary>
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexes_the_guide() {
        assert!(GUIDE.iter().any(|entry| entry.title == "Views"));
        let handlers = GUIDE
            .iter()
            .find(|entry| entry.title == "Views › Inline handlers")
            .unwrap();
        assert!(handlers.url.ends_with("views.html#inline-handlers"));
        assert!(!handlers.summary.is_empty());
    }

    #[test]
    fn ranks_prefixes_first() {
        let hits = search("view");
        assert_eq!(hits[0].1.title, "Views");
        assert!(hits.iter().any(|(_, entry)| entry.title == "#[view]"));
        assert!(search("zzzz").is_empty());
        assert!(
            search("")
                .iter()
                .all(|(_, entry)| !entry.title.contains(" › "))
        );
    }
}
