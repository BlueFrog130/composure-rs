use crate::types::{
    ApplicationCommandInteraction, Interaction, InteractionResponse, MessageComponentInteraction,
    ModalSubmitInteraction,
};

use super::{CommandSet, CommandSetDefinition, HandlerError, ScopedDefinition};

/// Answers Interactions; the one contract every Adapter talks to. `S` is the Adapter's State.
///
/// A Command Set implements this for commands. Implement it directly for components, modals,
/// autocomplete, or full control. Every method has a default: unknown command, a "not handled"
/// ephemeral message, or no autocomplete choices.
#[allow(async_fn_in_trait)] // dispatch is static and Workers futures are not Send
pub trait InteractionHandler<S> {
    async fn command(
        &self,
        command: ApplicationCommandInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        Err(HandlerError::unknown_command(&command.data.name))
    }

    async fn component(
        &self,
        _component: MessageComponentInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        Ok(not_handled("This bot does not handle components."))
    }

    /// Autocomplete for command options. Defaults to no choices.
    async fn autocomplete(
        &self,
        _autocomplete: ApplicationCommandInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        Ok(InteractionResponse::respond_with_autocomplete_choices(
            Vec::new(),
        ))
    }

    async fn modal_submit(
        &self,
        _modal: ModalSubmitInteraction,
        _state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        Ok(not_handled("This bot does not handle modal submissions."))
    }
}

fn not_handled(message: &str) -> InteractionResponse {
    InteractionResponse::respond_with_ephemeral_message(message.to_string())
}

/// Handles nothing: every Interaction gets the [`InteractionHandler`] defaults
#[derive(Debug, Clone, Copy, Default)]
pub struct NoHandler;

impl<S> InteractionHandler<S> for NoHandler {}

/// Routes an Interaction to the handler. Ping is answered with Pong.
pub async fn dispatch<S, H: InteractionHandler<S>>(
    interaction: Interaction,
    handler: &H,
    state: &S,
) -> Result<InteractionResponse, HandlerError> {
    match interaction {
        Interaction::Ping(_) => Ok(InteractionResponse::Pong),
        Interaction::ApplicationCommand(command) => handler.command(command, state).await,
        Interaction::MessageComponent(component) => handler.component(component, state).await,
        Interaction::ApplicationCommandAutocomplete(autocomplete) => {
            handler.autocomplete(autocomplete, state).await
        }
        Interaction::ModalSubmit(modal) => handler.modal_submit(modal, state).await,
    }
}

/// A Command Set plus a handler for everything it does not handle: components and modals
/// without a matching handler, autocomplete without an Autocomplete Handler, and commands not
/// in the set. Built with the set's `with_fallback`.
#[derive(Debug, Clone, Copy)]
pub struct WithFallback<C, F> {
    commands: C,
    fallback: F,
}

impl<C, F> WithFallback<C, F> {
    pub fn new(commands: C, fallback: F) -> Self {
        Self { commands, fallback }
    }

    pub fn commands(&self) -> &C {
        &self.commands
    }
}

impl<S, C: CommandSet<S>, F: InteractionHandler<S>> InteractionHandler<S> for WithFallback<C, F> {
    async fn command(
        &self,
        command: ApplicationCommandInteraction,
        state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        match self.commands.dispatch(&command, state).await {
            Some(result) => result,
            None => self.fallback.command(command, state).await,
        }
    }

    async fn component(
        &self,
        component: MessageComponentInteraction,
        state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        match self.commands.dispatch_component(&component, state).await {
            Some(result) => result,
            None => self.fallback.component(component, state).await,
        }
    }

    async fn autocomplete(
        &self,
        autocomplete: ApplicationCommandInteraction,
        state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        match self
            .commands
            .dispatch_autocomplete(&autocomplete, state)
            .await
        {
            Some(result) => result,
            None => self.fallback.autocomplete(autocomplete, state).await,
        }
    }

    async fn modal_submit(
        &self,
        modal: ModalSubmitInteraction,
        state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        match self.commands.dispatch_modal(&modal, state).await {
            Some(result) => result,
            None => self.fallback.modal_submit(modal, state).await,
        }
    }
}

impl<C: CommandSetDefinition, F> CommandSetDefinition for WithFallback<C, F> {
    fn definitions() -> Vec<ScopedDefinition> {
        C::definitions()
    }
}
