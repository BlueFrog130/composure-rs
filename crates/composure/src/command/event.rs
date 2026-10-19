//! Event Handlers: answering Webhook Events (ADR 0008).

use serde_json::value::RawValue;

use crate::types::{
    ApplicationAuthorized, ApplicationDeauthorized, Entitlement, Event, EventBody, LobbyMessage,
    LobbyMessageDelete, Snowflake,
};

use super::{HandlerError, NoHandler, WithFallback};

/// What an Event Handler sees besides the event: the Adapter's State and the event's envelope
pub struct EventCtx<'a, S> {
    state: &'a S,
    application_id: Snowflake,
    timestamp: &'a str,
}

impl<S> Clone for EventCtx<'_, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for EventCtx<'_, S> {}

impl<'a, S> EventCtx<'a, S> {
    pub fn new(state: &'a S, application_id: Snowflake, timestamp: &'a str) -> Self {
        Self {
            state,
            application_id,
            timestamp,
        }
    }

    pub fn state(&self) -> &'a S {
        self.state
    }

    pub fn application_id(&self) -> Snowflake {
        self.application_id
    }

    /// When the event happened (ISO 8601); events can arrive late and out of order
    pub fn timestamp(&self) -> &'a str {
        self.timestamp
    }
}

#[cfg(feature = "rest")]
impl<S: super::Platform> EventCtx<'_, S> {
    /// Client authorized by the bot token; an error when the Adapter has none
    pub fn bot(&self) -> Result<crate::rest::BotClient<S::Transport>, HandlerError> {
        let token = self
            .state
            .bot_token()
            .ok_or_else(|| HandlerError::new("the bot token (DISCORD_TOKEN) is not configured"))?;
        Ok(crate::rest::BotClient::new(
            self.state.transport(),
            &token,
            self.application_id,
        ))
    }
}

/// Answers Webhook Events. Every method defaults to ignoring the event; override the ones you
/// care about, or [`handle`](Self::handle) to see every event. Registered on a Command Set with
/// `commands! { ...; events = MyEvents; }`.
///
/// Events are acknowledged before they are handled, so errors are only logged, and Discord does
/// not retry them.
#[allow(async_fn_in_trait)] // dispatch is static and Workers futures are not Send
pub trait EventHandler<S> {
    /// Routes `event` to its method
    async fn handle(&self, ctx: EventCtx<'_, S>, event: Event) -> Result<(), HandlerError> {
        match event {
            Event::ApplicationAuthorized(e) => self.application_authorized(ctx, e).await,
            Event::ApplicationDeauthorized(e) => self.application_deauthorized(ctx, e).await,
            Event::EntitlementCreate(e) => self.entitlement_create(ctx, e).await,
            Event::EntitlementUpdate(e) => self.entitlement_update(ctx, e).await,
            Event::EntitlementDelete(e) => self.entitlement_delete(ctx, e).await,
            Event::LobbyMessageCreate(e) => self.lobby_message_create(ctx, e).await,
            Event::LobbyMessageUpdate(e) => self.lobby_message_update(ctx, e).await,
            Event::LobbyMessageDelete(e) => self.lobby_message_delete(ctx, e).await,
            Event::Unknown { kind, data } => self.unknown(ctx, &kind, data.as_deref()).await,
        }
    }

    async fn application_authorized(
        &self,
        _ctx: EventCtx<'_, S>,
        _event: ApplicationAuthorized,
    ) -> Result<(), HandlerError> {
        Ok(())
    }

    async fn application_deauthorized(
        &self,
        _ctx: EventCtx<'_, S>,
        _event: ApplicationDeauthorized,
    ) -> Result<(), HandlerError> {
        Ok(())
    }

    async fn entitlement_create(
        &self,
        _ctx: EventCtx<'_, S>,
        _entitlement: Entitlement,
    ) -> Result<(), HandlerError> {
        Ok(())
    }

    async fn entitlement_update(
        &self,
        _ctx: EventCtx<'_, S>,
        _entitlement: Entitlement,
    ) -> Result<(), HandlerError> {
        Ok(())
    }

    async fn entitlement_delete(
        &self,
        _ctx: EventCtx<'_, S>,
        _entitlement: Entitlement,
    ) -> Result<(), HandlerError> {
        Ok(())
    }

    async fn lobby_message_create(
        &self,
        _ctx: EventCtx<'_, S>,
        _message: LobbyMessage,
    ) -> Result<(), HandlerError> {
        Ok(())
    }

    async fn lobby_message_update(
        &self,
        _ctx: EventCtx<'_, S>,
        _message: LobbyMessage,
    ) -> Result<(), HandlerError> {
        Ok(())
    }

    async fn lobby_message_delete(
        &self,
        _ctx: EventCtx<'_, S>,
        _message: LobbyMessageDelete,
    ) -> Result<(), HandlerError> {
        Ok(())
    }

    /// Event types without a method (game DMs, quests, ones added later), or whose data did not
    /// match its model; `data` is the raw JSON
    async fn unknown(
        &self,
        _ctx: EventCtx<'_, S>,
        _kind: &str,
        _data: Option<&RawValue>,
    ) -> Result<(), HandlerError> {
        Ok(())
    }
}

impl<S> EventHandler<S> for NoHandler {}

impl<S, C: EventHandler<S>, F> EventHandler<S> for WithFallback<C, F> {
    async fn handle(&self, ctx: EventCtx<'_, S>, event: Event) -> Result<(), HandlerError> {
        self.commands().handle(ctx, event).await
    }
}

/// Parses and handles one event
pub async fn dispatch_event<S, H: EventHandler<S>>(
    handler: &H,
    state: &S,
    application_id: Snowflake,
    body: &EventBody,
) -> Result<(), HandlerError> {
    let ctx = EventCtx::new(state, application_id, &body.timestamp);
    handler.handle(ctx, body.parse()).await
}

/// Hands one event to `handler` as Background Work, so the Adapter can acknowledge it at once
#[cfg(feature = "rest")]
pub fn spawn_event<S: super::Platform, H: EventHandler<S> + 'static>(
    handler: H,
    state: &S,
    application_id: Snowflake,
    body: EventBody,
) {
    use crate::trace::Instrument;

    let owned = state.clone();
    let span = info_span!("event", event_type = %body.kind);
    let work = async move {
        debug!("event received");
        let result = dispatch_event(&handler, &owned, application_id, &body).await;
        #[cfg(feature = "tracing")]
        match &result {
            Ok(()) => debug!("event handled"),
            Err(error) => error.trace("event failed"),
        }
        result
    }
    .instrument(span.clone());
    span.in_scope(|| state.spawn(Box::pin(work)));
}

/// Implements [`EventHandler`] for a Command Set: delegating to `$events`, or ignoring every
/// event. Used by [`commands!`](crate::commands).
#[doc(hidden)]
#[macro_export]
macro_rules! __command_set_events {
    ($name:ident) => {
        impl<S> $crate::command::EventHandler<S> for $name {}
    };
    ($name:ident, $events:path) => {
        impl<S> $crate::command::EventHandler<S> for $name
        where
            $events: $crate::command::EventHandler<S>,
        {
            async fn handle(
                &self,
                ctx: $crate::command::EventCtx<'_, S>,
                event: $crate::types::Event,
            ) -> ::core::result::Result<(), $crate::command::HandlerError> {
                $crate::command::EventHandler::<S>::handle(&$events, ctx, event).await
            }
        }
    };
}
