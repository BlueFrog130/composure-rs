use std::sync::Arc;

use crate::trace::{Instrument, Span};
use crate::types::{
    ApplicationCommandInteraction, Interaction, InteractionResponse, MessageComponentInteraction,
    ModalSubmitInteraction,
};

use super::{__unshare, CommandSetDefinition, HandlerError, Route, ScopedDefinition, Theme};

/// Answers Interactions; the one contract every Adapter talks to. `S` is the Adapter's State.
///
/// An `#[app]` implements this by routing to what it lists. Implement it directly for full
/// control. Every method has a default: unknown command, a "not handled"
/// ephemeral message, or no autocomplete choices.
///
/// Futures are `Send` so Adapters can run each Interaction on any thread (ADR 0012); `async fn`
/// in impls works as long as what it holds across `.await` is `Send`.
pub trait InteractionHandler<S> {
    /// Colors and branding filled into message responses and errors; none by default
    fn theme(&self) -> Option<&'static Theme> {
        None
    }

    /// Answers a Command (chat input, user, or message); defaults to an unknown-command error
    fn command(
        &self,
        command: ApplicationCommandInteraction,
        _state: &S,
    ) -> impl Future<Output = Result<InteractionResponse, HandlerError>> + Send {
        async move { Err(HandlerError::unknown_command(&command.data.name)) }
    }

    /// Answers a button click or select; defaults to a "not handled" ephemeral message
    fn component(
        &self,
        _component: MessageComponentInteraction,
        _state: &S,
    ) -> impl Future<Output = Result<InteractionResponse, HandlerError>> + Send {
        async { Ok(not_handled("This bot does not handle components.")) }
    }

    /// Autocomplete for command options. Defaults to no choices.
    fn autocomplete(
        &self,
        _autocomplete: ApplicationCommandInteraction,
        _state: &S,
    ) -> impl Future<Output = Result<InteractionResponse, HandlerError>> + Send {
        async {
            Ok(InteractionResponse::respond_with_autocomplete_choices(
                Vec::new(),
            ))
        }
    }

    /// Answers a modal submit; defaults to a "not handled" ephemeral message
    fn modal_submit(
        &self,
        _modal: ModalSubmitInteraction,
        _state: &S,
    ) -> impl Future<Output = Result<InteractionResponse, HandlerError>> + Send {
        async { Ok(not_handled("This bot does not handle modal submissions.")) }
    }
}

fn not_handled(message: &str) -> InteractionResponse {
    InteractionResponse::respond_with_ephemeral_message(message.to_string())
}

/// Handles nothing: every Interaction gets the [`InteractionHandler`] defaults
#[derive(Debug, Clone, Copy, Default)]
pub struct NoHandler;

impl<S> InteractionHandler<S> for NoHandler {}

/// Routes an Interaction to the handler. Ping is answered with Pong. The handler's
/// [`InteractionHandler::theme`] is applied to the response; render an `Err` with
/// [`HandlerError::response_with`] and the same theme.
pub async fn dispatch<S, H: InteractionHandler<S>>(
    interaction: Interaction,
    handler: &H,
    state: &S,
) -> Result<InteractionResponse, HandlerError> {
    let span = span(&interaction);
    async move {
        debug!("interaction received");
        let mut response = match route(interaction, handler, state).await {
            Ok(response) => response,
            Err(error) => {
                error.trace("interaction failed");
                return Err(error);
            }
        };
        if let Some(theme) = handler.theme() {
            theme.apply(&mut response);
        }
        debug!(response = response_kind(&response), "interaction answered");
        Ok(response)
    }
    .instrument(span)
    .await
}

/// The response's Discord type, as logged by [`dispatch`]
#[cfg(feature = "tracing")]
fn response_kind(response: &InteractionResponse) -> &'static str {
    match response {
        InteractionResponse::Pong => "pong",
        InteractionResponse::ChannelMessageWithSource(_) => "message",
        InteractionResponse::DeferredChannelMessageWithSource(_) => "deferred_message",
        InteractionResponse::DeferredUpdateMessage => "deferred_update",
        InteractionResponse::UpdateMessage(_) => "update",
        InteractionResponse::ApplicationCommandAutocompleteResult(_) => "autocomplete",
        InteractionResponse::Modal(_) => "modal",
    }
}

/// The `interaction` span: what was invoked, where, by whom. Components and modals record their
/// `custom_id` prefix only; the rest is state.
#[cfg(feature = "tracing")]
fn span(interaction: &Interaction) -> Span {
    let (kind, common, name) = match interaction {
        Interaction::Ping(ping) => ("ping", &ping.common, None),
        Interaction::ApplicationCommand(command) => {
            ("command", &command.common, Some(command.data.name.as_str()))
        }
        Interaction::ApplicationCommandAutocomplete(autocomplete) => (
            "autocomplete",
            &autocomplete.common,
            Some(autocomplete.data.name.as_str()),
        ),
        Interaction::MessageComponent(component) => (
            "component",
            &component.common,
            Some(super::custom_id::split(&component.data.custom_id).0),
        ),
        Interaction::ModalSubmit(modal) => (
            "modal_submit",
            &modal.common,
            Some(super::custom_id::split(&modal.data.custom_id).0),
        ),
    };
    info_span!(
        "interaction",
        interaction_id = %common.id,
        interaction_type = kind,
        name,
        guild_id = common.guild_id.as_ref().map(tracing::field::display),
        channel_id = common.channel_id.as_ref().map(tracing::field::display),
        user_id = common.invoking_user().map(|user| tracing::field::display(&user.id)),
    )
}

#[cfg(not(feature = "tracing"))]
fn span(_interaction: &Interaction) -> Span {
    Span
}

async fn route<S, H: InteractionHandler<S>>(
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

/// An App plus a handler for everything it does not handle: components and modals without a
/// matching handler, autocomplete without an Autocomplete Handler, and commands it does not
/// list. Built with the App's `with_fallback`.
#[derive(Debug, Clone, Copy)]
pub struct WithFallback<C, F> {
    commands: C,
    fallback: F,
}

impl<C, F> WithFallback<C, F> {
    /// `commands` (an App) first, `fallback` for the rest; usually built with the App's
    /// `with_fallback`
    pub fn new(commands: C, fallback: F) -> Self {
        Self { commands, fallback }
    }

    /// The App
    pub fn commands(&self) -> &C {
        &self.commands
    }
}

impl<S: Sync, C: Route<S> + Sync, F: InteractionHandler<S> + Sync> InteractionHandler<S>
    for WithFallback<C, F>
{
    fn theme(&self) -> Option<&'static Theme> {
        C::THEME
    }

    async fn command(
        &self,
        command: ApplicationCommandInteraction,
        state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        let command = Arc::new(command);
        match C::dispatch(&command, state, None).await {
            Some(result) => result,
            None => self.fallback.command(__unshare(command), state).await,
        }
    }

    async fn component(
        &self,
        component: MessageComponentInteraction,
        state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        let component = Arc::new(component);
        match C::dispatch_component(&component, state, None).await {
            Some(result) => result,
            None => self.fallback.component(__unshare(component), state).await,
        }
    }

    async fn autocomplete(
        &self,
        autocomplete: ApplicationCommandInteraction,
        state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        let autocomplete = Arc::new(autocomplete);
        match C::dispatch_autocomplete(&autocomplete, state, None).await {
            Some(result) => result,
            None => {
                self.fallback
                    .autocomplete(__unshare(autocomplete), state)
                    .await
            }
        }
    }

    async fn modal_submit(
        &self,
        modal: ModalSubmitInteraction,
        state: &S,
    ) -> Result<InteractionResponse, HandlerError> {
        let modal = Arc::new(modal);
        match C::dispatch_modal(&modal, state, None).await {
            Some(result) => result,
            None => self.fallback.modal_submit(__unshare(modal), state).await,
        }
    }
}

impl<C: CommandSetDefinition, F> CommandSetDefinition for WithFallback<C, F> {
    fn definitions() -> Vec<ScopedDefinition> {
        C::definitions()
    }
}
