use crate::types::{ApplicationCommandInteraction, Member, Snowflake, User};

/// What a Command sees besides its Options: the Interaction it answers and the Adapter's State.
pub struct Ctx<'a, S = ()> {
    interaction: &'a ApplicationCommandInteraction,
    state: &'a S,
}

impl<S> Clone for Ctx<'_, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Ctx<'_, S> {}

impl<'a, S> Ctx<'a, S> {
    pub fn new(interaction: &'a ApplicationCommandInteraction, state: &'a S) -> Self {
        Self { interaction, state }
    }

    pub fn interaction(&self) -> &'a ApplicationCommandInteraction {
        self.interaction
    }

    /// Platform resources passed through by the Adapter (e.g. `worker::Env` on Cloudflare)
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
}
