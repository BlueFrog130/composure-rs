use std::sync::Arc;

use super::Theme;
use crate::types::{
    ApplicationCommandInteractionData, DataInteraction, Entitlement, Member, MessageComponentData,
    ModalSubmitData, Snowflake, User,
};

/// What a handler sees besides its inputs: the Interaction it answers and the Adapter's State.
/// `D` is the Interaction's data: a Command's by default, see [`ComponentCtx`] and [`ModalCtx`].
pub struct Ctx<'a, S = (), D = ApplicationCommandInteractionData> {
    /// Shared so deferred work keeps the Interaction without copying it
    interaction: &'a Arc<DataInteraction<D>>,
    state: &'a S,
    theme: Option<&'static Theme>,
}

/// [`Ctx`] of a Component Handler: the clicked component's data (custom_id, select values)
pub type ComponentCtx<'a, S = ()> = Ctx<'a, S, MessageComponentData>;

/// [`Ctx`] of a Modal Handler: the submitted modal's data
pub type ModalCtx<'a, S = ()> = Ctx<'a, S, ModalSubmitData>;

impl<S, D> Clone for Ctx<'_, S, D> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S, D> Copy for Ctx<'_, S, D> {}

impl<'a, S, D> Ctx<'a, S, D> {
    /// A context for `interaction` with the Adapter's `state`, without a theme. Dispatch builds
    /// these; build one by hand to call a handler directly.
    pub fn new(interaction: &'a Arc<DataInteraction<D>>, state: &'a S) -> Self {
        Self {
            interaction,
            state,
            theme: None,
        }
    }

    /// This Ctx with the App's theme; `None` leaves responses unthemed
    pub fn with_theme(mut self, theme: Option<&'static Theme>) -> Self {
        self.theme = theme;
        self
    }

    /// The App's colors and branding: `ctx.theme().success("Saved")`.
    /// [`Theme::DEFAULT`] when the App has none.
    pub fn theme(&self) -> &'static Theme {
        self.theme.unwrap_or(&Theme::DEFAULT)
    }

    /// The App's theme, `None` when it has none
    #[cfg(feature = "rest")]
    pub(crate) fn configured_theme(&self) -> Option<&'static Theme> {
        self.theme
    }

    /// The Interaction being answered: ids, token, user, member, guild, channel, data
    pub fn interaction(&self) -> &'a DataInteraction<D> {
        self.interaction
    }

    /// The Interaction, shared
    #[cfg(feature = "rest")]
    pub(crate) fn shared_interaction(&self) -> &'a Arc<DataInteraction<D>> {
        self.interaction
    }

    /// The Interaction's data: options for Commands, custom_id and values for components,
    /// submitted fields for modals
    pub fn data(&self) -> &'a D {
        &self.interaction.data
    }

    /// Platform resources passed through by the Adapter (e.g. `Worker` on Cloudflare)
    pub fn state(&self) -> &'a S {
        self.state
    }

    /// The invoking user, in guilds and DMs alike
    pub fn user(&self) -> Option<&'a User> {
        self.interaction.common.invoking_user()
    }

    /// The invoking member, in guilds only
    pub fn member(&self) -> Option<&'a Member> {
        self.interaction.common.member.as_ref()
    }

    /// The guild it was invoked in; `None` in DMs
    pub fn guild_id(&self) -> Option<&'a Snowflake> {
        self.interaction.common.guild_id.as_ref()
    }

    /// The channel it was invoked in
    pub fn channel_id(&self) -> Option<&'a Snowflake> {
        self.interaction.common.channel_id.as_ref()
    }

    /// The invoking user's selected language
    pub fn locale(&self) -> Option<&'a str> {
        self.interaction.locale.as_deref()
    }

    /// The invoking user's and guild's [entitlements](https://discord.com/developers/docs/resources/entitlement)
    /// to the app's premium SKUs
    pub fn entitlements(&self) -> &'a [Entitlement] {
        &self.interaction.common.entitlements
    }

    /// Whether the user or guild is entitled to `sku_id` right now (not deleted or consumed)
    pub fn is_entitled(&self, sku_id: Snowflake) -> bool {
        self.entitlements()
            .iter()
            .any(|e| e.sku_id == sku_id && !e.deleted && e.consumed != Some(true))
    }
}

impl<'a, S> ComponentCtx<'a, S> {
    /// Selected values of a select menu: option values, or user/role/channel ids
    pub fn values(&self) -> &'a [String] {
        self.interaction.data.values.as_deref().unwrap_or_default()
    }
}

impl<'a, S> ModalCtx<'a, S> {
    /// The submitted text of the text input or radio group with `custom_id`
    pub fn value(&self, custom_id: &str) -> Option<&'a str> {
        self.interaction.data.value(custom_id)
    }

    /// The submitted values of the select, checkbox group, or file upload with `custom_id`
    pub fn values(&self, custom_id: &str) -> &'a [String] {
        self.interaction.data.values(custom_id).unwrap_or_default()
    }

    /// Whether the checkbox with `custom_id` was checked
    pub fn checked(&self, custom_id: &str) -> Option<bool> {
        self.interaction.data.checked(custom_id)
    }
}
