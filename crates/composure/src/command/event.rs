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
    /// The context of an event for `application_id` that happened at `timestamp`
    pub fn new(state: &'a S, application_id: Snowflake, timestamp: &'a str) -> Self {
        Self {
            state,
            application_id,
            timestamp,
        }
    }

    /// Platform resources passed through by the Adapter
    pub fn state(&self) -> &'a S {
        self.state
    }

    /// The application the event is for
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
        super::platform::bot_client(self.state, self.application_id)
    }
}

/// Answers Webhook Events. Every method defaults to ignoring the event; override the ones you
/// care about, or [`handle`](Self::handle) to see every event. Registered on an App with
/// `#[app(.., events = MyEvents)]`.
///
/// Events are acknowledged before they are handled, so errors are only logged, and Discord does
/// not retry them. Futures are `Send` so they can run on any thread (ADR 0012).
pub trait EventHandler<S: Sync>: Sync {
    /// Routes `event` to its method
    fn handle(
        &self,
        ctx: EventCtx<'_, S>,
        event: Event,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async move {
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
    }

    /// The app was installed to a server or a user account
    fn application_authorized(
        &self,
        _ctx: EventCtx<'_, S>,
        _event: ApplicationAuthorized,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async { Ok(()) }
    }

    /// A user removed the app from their account
    fn application_deauthorized(
        &self,
        _ctx: EventCtx<'_, S>,
        _event: ApplicationDeauthorized,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async { Ok(()) }
    }

    /// A user or server bought one of the app's SKUs
    fn entitlement_create(
        &self,
        _ctx: EventCtx<'_, S>,
        _entitlement: Entitlement,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async { Ok(()) }
    }

    /// An entitlement changed, e.g. a subscription renewed or ended
    fn entitlement_update(
        &self,
        _ctx: EventCtx<'_, S>,
        _entitlement: Entitlement,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async { Ok(()) }
    }

    /// An entitlement was removed, e.g. refunded
    fn entitlement_delete(
        &self,
        _ctx: EventCtx<'_, S>,
        _entitlement: Entitlement,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async { Ok(()) }
    }

    /// A message was sent in a lobby the app owns
    fn lobby_message_create(
        &self,
        _ctx: EventCtx<'_, S>,
        _message: LobbyMessage,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async { Ok(()) }
    }

    /// A lobby message was edited
    fn lobby_message_update(
        &self,
        _ctx: EventCtx<'_, S>,
        _message: LobbyMessage,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async { Ok(()) }
    }

    /// A lobby message was deleted
    fn lobby_message_delete(
        &self,
        _ctx: EventCtx<'_, S>,
        _message: LobbyMessageDelete,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async { Ok(()) }
    }

    /// Event types without a method (game DMs, quests, ones added later), or whose data did not
    /// match its model; `data` is the raw JSON
    fn unknown(
        &self,
        _ctx: EventCtx<'_, S>,
        _kind: &str,
        _data: Option<&RawValue>,
    ) -> impl Future<Output = Result<(), HandlerError>> + Send {
        async { Ok(()) }
    }
}

impl<S: Sync> EventHandler<S> for NoHandler {}

impl<S: Sync, C: EventHandler<S>, F: Sync> EventHandler<S> for WithFallback<C, F> {
    async fn handle(&self, ctx: EventCtx<'_, S>, event: Event) -> Result<(), HandlerError> {
        self.commands().handle(ctx, event).await
    }
}

/// Parses and handles one event
pub async fn dispatch_event<S: Sync, H: EventHandler<S>>(
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
pub fn spawn_event<S: super::Platform, H: EventHandler<S> + Send + 'static>(
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
