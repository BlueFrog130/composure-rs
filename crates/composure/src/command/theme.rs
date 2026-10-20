//! Consistent embed colors and branding for everything the bot says.

use crate::types::{Embed, EmbedAuthor, EmbedFooter, InteractionResponse};

/// A bot's colors and branding, set on an App with `#[app(.., theme = THEME)]` and read with
/// `ctx.theme()`. An App listed in another uses its own Theme, if set, and fills in the rest from
/// the outer one; errors are rendered with the outermost App's.
///
/// ```
/// use composure::command::Theme;
///
/// const THEME: Theme = Theme::new()
///     .with_primary(0xf5c2e7)
///     .with_author("Composure", Some("https://example.com/icon.png"))
///     .with_footer("composure-rs", None);
///
/// let saved = THEME.success("Saved");
/// assert_eq!(saved.color, Some(THEME.success));
/// ```
///
/// Message responses (and deferred edits) of a themed App are filled in where unset:
/// every embed gets `primary`, the first the author, the last the footer; top-level
/// [`Container`](crate::types::Container)s get `primary` as accent color. Errors become an
/// `error`-colored embed. Wrap a response in [`Unthemed`](super::Unthemed) to send it as is.
/// Messages sent through REST are never changed: build them with the helpers.
///
/// See the [Themes chapter](crate::guide::theming) of the guide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    /// Embeds without a semantic color, auto-filled embeds and container accents (`0xRRGGBB`)
    pub primary: u32,
    /// [`Theme::success`] embeds
    pub success: u32,
    /// [`Theme::warning`] embeds
    pub warning: u32,
    /// [`Theme::error`] embeds, and the error messages users see when a handler fails
    pub error: u32,
    /// [`Theme::info`] embeds
    pub info: u32,
    /// Set on the first embed of a themed message, and on every helper's embed
    pub author: Option<Branding>,
    /// Set on the last embed of a themed message, and on every helper's embed
    pub footer: Option<Branding>,
}

/// Text and optional icon url of a [`Theme`]'s author or footer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Branding {
    /// The author's name, or the footer's text
    pub text: &'static str,
    /// A small image shown before the text
    pub icon_url: Option<&'static str>,
}

impl Default for Theme {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Theme {
    /// Discord's palette, no branding. What `ctx.theme()` returns when none is set.
    pub const DEFAULT: Theme = Theme {
        primary: 0x5865F2,
        success: 0x57F287,
        warning: 0xFEE75C,
        error: 0xED4245,
        info: 0x5865F2,
        author: None,
        footer: None,
    };

    /// [`Theme::DEFAULT`], to customize with the builder methods
    pub const fn new() -> Self {
        Self::DEFAULT
    }

    /// Sets [`primary`](Theme::primary): the default color of embeds and container accents
    pub const fn with_primary(mut self, color: u32) -> Self {
        self.primary = color;
        self
    }

    /// Sets [`success`](Theme::success)
    pub const fn with_success(mut self, color: u32) -> Self {
        self.success = color;
        self
    }

    /// Sets [`warning`](Theme::warning)
    pub const fn with_warning(mut self, color: u32) -> Self {
        self.warning = color;
        self
    }

    /// Sets [`error`](Theme::error), also used for handler error messages
    pub const fn with_error(mut self, color: u32) -> Self {
        self.error = color;
        self
    }

    /// Sets [`info`](Theme::info)
    pub const fn with_info(mut self, color: u32) -> Self {
        self.info = color;
        self
    }

    /// Author shown at the top of themed embeds
    pub const fn with_author(mut self, name: &'static str, icon_url: Option<&'static str>) -> Self {
        self.author = Some(Branding {
            text: name,
            icon_url,
        });
        self
    }

    /// Footer shown at the bottom of themed embeds
    pub const fn with_footer(mut self, text: &'static str, icon_url: Option<&'static str>) -> Self {
        self.footer = Some(Branding { text, icon_url });
        self
    }

    /// An empty embed with the primary color and the branding
    pub fn embed(&self) -> Embed {
        let mut embed = Embed::new();
        embed.color = Some(self.primary);
        embed.author = self.author.map(Branding::embed_author);
        embed.footer = self.footer.map(Branding::embed_footer);
        embed
    }

    /// A branded embed with `message` in the success color
    pub fn success(&self, message: impl Into<String>) -> Embed {
        self.message(self.success, message.into())
    }

    /// A branded embed with `message` in the warning color
    pub fn warning(&self, message: impl Into<String>) -> Embed {
        self.message(self.warning, message.into())
    }

    /// A branded embed with `message` in the error color
    pub fn error(&self, message: impl Into<String>) -> Embed {
        self.message(self.error, message.into())
    }

    /// A branded embed with `message` in the info color
    pub fn info(&self, message: impl Into<String>) -> Embed {
        self.message(self.info, message.into())
    }

    fn message(&self, color: u32, message: String) -> Embed {
        let mut embed = self.embed();
        embed.color = Some(color);
        embed.description = Some(message);
        embed
    }

    /// Fills unset colors and branding on a message response: the primary color on every embed
    /// and as accent of top-level containers, the author on the first embed, the footer on the
    /// last. Other responses and [`Unthemed`](super::Unthemed) messages are left as is.
    pub fn apply(&self, response: &mut InteractionResponse) {
        let Some(data) = response.message_data_mut() else {
            return;
        };
        if data.unthemed {
            return;
        }
        for container in data.containers_mut() {
            container.accent_color.get_or_insert(self.primary);
        }
        let Some(embeds) = data.embeds.as_mut() else {
            return;
        };
        for embed in embeds.iter_mut() {
            embed.color.get_or_insert(self.primary);
        }
        if let (Some(first), Some(author)) = (embeds.first_mut(), self.author) {
            first.author.get_or_insert_with(|| author.embed_author());
        }
        if let (Some(last), Some(footer)) = (embeds.last_mut(), self.footer) {
            last.footer.get_or_insert_with(|| footer.embed_footer());
        }
    }
}

impl Branding {
    fn embed_author(self) -> EmbedAuthor {
        EmbedAuthor::new(
            self.text.to_string(),
            None,
            self.icon_url.map(str::to_string),
            None,
        )
    }

    fn embed_footer(self) -> EmbedFooter {
        EmbedFooter::new(
            self.text.to_string(),
            self.icon_url.map(str::to_string),
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Ephemeral, IntoResponse, Unthemed};
    use crate::types::{Component, Container, MessageFlags};

    const THEME: Theme = Theme::new()
        .with_primary(0x111111)
        .with_author("Bot", Some("https://example.com/a.png"))
        .with_footer("Footer", None);

    fn embeds(response: &InteractionResponse) -> &[Embed] {
        response.message_data().unwrap().embeds.as_deref().unwrap()
    }

    #[test]
    fn helpers_are_branded_and_colored() {
        let embed = THEME.warning("careful");
        assert_eq!(embed.color, Some(Theme::DEFAULT.warning));
        assert_eq!(embed.description.as_deref(), Some("careful"));
        assert_eq!(embed.author.unwrap().name, "Bot");
        assert_eq!(embed.footer.unwrap().text, "Footer");
        assert_eq!(THEME.embed().color, Some(0x111111));
    }

    #[test]
    fn apply_fills_only_unset_fields() {
        let mut response = Embed::new().with_color(0x222222).into_response();
        THEME.apply(&mut response);
        let embed = &embeds(&response)[0];
        assert_eq!(embed.color, Some(0x222222));
        assert_eq!(embed.author.as_ref().unwrap().name, "Bot");
        assert_eq!(
            embed.author.as_ref().unwrap().icon_url.as_deref(),
            Some("https://example.com/a.png")
        );
        assert_eq!(embed.footer.as_ref().unwrap().text, "Footer");
    }

    #[test]
    fn apply_puts_author_first_and_footer_last() {
        let mut response = InteractionResponse::respond_with_embed(Embed::new());
        response.message_data_mut().unwrap().embeds = Some(vec![Embed::new(); 3]);
        THEME.apply(&mut response);
        let embeds = embeds(&response);
        assert!(embeds.iter().all(|e| e.color == Some(0x111111)));
        let authors: Vec<_> = embeds.iter().map(|e| e.author.is_some()).collect();
        let footers: Vec<_> = embeds.iter().map(|e| e.footer.is_some()).collect();
        assert_eq!(authors, [true, false, false]);
        assert_eq!(footers, [false, false, true]);
    }

    fn accents(response: &InteractionResponse) -> Vec<Option<u32>> {
        let components = response.message_data().unwrap().components.as_deref();
        components
            .unwrap()
            .iter()
            .filter_map(|c| match c {
                Component::Container(container) => Some(container.accent_color),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn apply_fills_unset_container_accents() {
        let mut response = InteractionResponse::respond_with_components(vec![
            Component::container(vec![Component::text("a")]),
            Container::new(vec![]).with_accent_color(0x222222).into(),
            Component::text("outside"),
        ]);
        THEME.apply(&mut response);
        assert_eq!(accents(&response), [Some(0x111111), Some(0x222222)]);
        let data = response.message_data().unwrap();
        assert!(data.is_components_v2());
        assert!(data.content.is_none() && data.embeds.is_none());
    }

    #[test]
    fn apply_skips_unthemed_containers() {
        let mut response = Ephemeral(Unthemed(Container::new(vec![]))).into_response();
        THEME.apply(&mut response);
        assert_eq!(accents(&response), [None]);
        let flags = response.message_data().unwrap().flags.unwrap();
        assert_eq!(
            flags,
            MessageFlags::IsComponentsV2 | MessageFlags::Ephemeral
        );
    }

    #[test]
    fn apply_skips_unthemed_and_plain_messages() {
        let mut response = Ephemeral(Unthemed(Embed::new())).into_response();
        THEME.apply(&mut response);
        assert_eq!(embeds(&response)[0].color, None);

        let mut text = "hi".into_response();
        THEME.apply(&mut text);
        assert!(text.message_data().unwrap().embeds.is_none());
    }
}
