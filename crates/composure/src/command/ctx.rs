use crate::types::{
    ApplicationCommandInteractionData, DataInteraction, Entitlement, Member, MessageComponentData,
    ModalSubmitData, Snowflake, User,
};

/// What a handler sees besides its inputs: the Interaction it answers and the Adapter's State.
/// `D` is the Interaction's data: a Command's by default, see [`ComponentCtx`] and [`ModalCtx`].
pub struct Ctx<'a, S = (), D = ApplicationCommandInteractionData> {
    interaction: &'a DataInteraction<D>,
    state: &'a S,
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
    pub fn new(interaction: &'a DataInteraction<D>, state: &'a S) -> Self {
        Self { interaction, state }
    }

    pub fn interaction(&self) -> &'a DataInteraction<D> {
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

    pub fn guild_id(&self) -> Option<&'a Snowflake> {
        self.interaction.common.guild_id.as_ref()
    }

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
    /// The submitted value of the text input with `custom_id`
    pub fn value(&self, custom_id: &str) -> Option<&'a str> {
        self.interaction.data.value(custom_id)
    }
}
