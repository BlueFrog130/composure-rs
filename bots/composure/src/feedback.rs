//! `/feedback`: a modal View whose submission is posted to the maintainers' channel, answered
//! with a link that opens the same report as a prefilled GitHub issue (no GitHub write access
//! needed).

use composure::{
    Choice, app, command,
    command::{Ephemeral, HandlerError, ModalCtx, ModalView, View},
    rsx, view,
};

use crate::{Services, ui};

/// Discord's limit on a button's URL
const URL_LIMIT: usize = 512;

/// What the feedback is
#[derive(Choice, Debug, Clone, Copy)]
pub enum Kind {
    Bug,
    #[choice(name = "Feature request")]
    Feature,
    Question,
}

/// The form; submitted to its inline handler
#[view(prefix = "fb")]
pub fn FeedbackForm() -> ModalView {
    rsx! {
        <Modal title="Feedback on Composure" on:submit:send={async move |ctx: ModalCtx<impl Services>, kind: Kind, title: String, details: String| {
            send(ctx, kind, title, details).await
        }}>
            <Label text="Kind">
                <RadioGroup name="kind" options=Kind/>
            </Label>
            <Label text="Title">
                <TextInput name="title" max_len=100 placeholder="One line about it"/>
            </Label>
            <Label text="Details" description="Steps, versions, what you expected">
                <TextInput name="details" paragraph max_len=1500/>
            </Label>
        </Modal>
    }
}

async fn send(
    ctx: ModalCtx<'_, impl Services>,
    kind: Kind,
    title: String,
    details: String,
) -> Result<Ephemeral<View>, HandlerError> {
    let user = ctx
        .user()
        .ok_or_else(|| HandlerError::new("interaction without a user"))?;
    let config = ctx.state().data();
    let title = format!("[{}] {title}", kind.name());
    let from = format!(
        "-# From <@{}> ({}){}",
        user.id,
        user.username,
        ctx.guild_id()
            .map(|guild| format!(" in server `{guild}`"))
            .unwrap_or_default()
    );
    let body = rsx! {
        <Text>{details.as_str()}</Text>
        <Text>{from}</Text>
    };
    // Bugs stand out in the maintainers' channel
    let report = match kind {
        Kind::Bug => rsx! { <ui::Error title=title.as_str()>{body}</ui::Error> },
        Kind::Feature => rsx! { <ui::Primary title=title.as_str()>{body}</ui::Primary> },
        Kind::Question => rsx! { <ui::Secondary title=title.as_str()>{body}</ui::Secondary> },
    };
    ui::send(&ctx.bot()?, config.feedback_channel, report).await?;
    let issue = issue_url(&config.repo_url(), &title, &details);
    Ok(Ephemeral(rsx! {
        <ui::Success title="Thanks!">
            <Text>"The maintainers got your feedback."</Text>
            <Text>"-# To follow it on GitHub, open it as an issue (prefilled; long details are shortened):"</Text>
            <Row>
                <Button url=issue>"Open as GitHub issue"</Button>
            </Row>
        </ui::Success>
    }))
}

/// A new-issue link with `title` and as much of `body` as fits Discord's URL limit
pub fn issue_url(repo_url: &str, title: &str, body: &str) -> String {
    let mut url = format!("{repo_url}/issues/new?title={}&body=", encode(title));
    // Room for an encoded "…" when cut
    let budget = URL_LIMIT.saturating_sub(url.len() + 9);
    let mut encoded = String::new();
    let mut cut = false;
    for c in body.chars() {
        let piece = encode(c.encode_utf8(&mut [0; 4]));
        if encoded.len() + piece.len() > budget {
            cut = true;
            break;
        }
        encoded.push_str(&piece);
    }
    url.push_str(&encoded);
    if cut {
        url.push_str("%E2%80%A6");
    }
    url
}

/// Percent-encodes everything but unreserved characters
fn encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Send feedback to Composure's maintainers
#[command(
    contexts(Guild, BotDm, PrivateChannel),
    integration_types(GuildInstall, UserInstall)
)]
pub fn feedback() -> FeedbackForm {
    FeedbackForm {}
}

/// The form and the Command opening it, listed in [`ComposureBot`](crate::ComposureBot)
#[app(handlers(FeedbackForm, feedback))]
pub struct Feedbacks;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefills_issues_within_the_limit() {
        let repo = "https://github.com/o/r";
        assert_eq!(
            issue_url(repo, "[Bug] a b", "x/y"),
            "https://github.com/o/r/issues/new?title=%5BBug%5D%20a%20b&body=x%2Fy"
        );
        let long = issue_url(repo, "t", &"é".repeat(500));
        assert!(long.len() <= URL_LIMIT);
        assert!(long.ends_with("%E2%80%A6"));
    }
}
