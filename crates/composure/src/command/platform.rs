//! REST and Background Work for handlers, when the Adapter's State provides them (ADR 0006).

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use crate::{
    rest::{BotClient, CreateFollowup, EditMessage, Error, InteractionClient, Transport},
    types::{
        ApplicationCommandInteractionData, DataInteraction, InteractionCommon, InteractionResponse,
        Member, MessageComponentData, MessageFlags, ModalSubmitData, Snowflake, User,
    },
};

use super::{CommandOutput, Ctx, HandlerError, IntoResponse, Theme, response::check_components_v2};
use crate::trace::Instrument;

/// Background Work handed to the Adapter: runs after the response is sent; an `Err` is logged.
/// `Send`, so Adapters can run it on any thread (ADR 0012).
pub type BackgroundWork = Pin<Box<dyn Future<Output = Result<(), HandlerError>> + Send>>;

/// What an Adapter's State offers beyond its own resources: a [`Transport`] for REST, a way to run
/// Background Work, and the bot token when configured. Implemented by each Adapter's State
/// (`composure_adapter_cloudflare::Worker`, `composure_adapter_tokio::Host`); enables
/// [`Ctx::client`], [`Ctx::bot`] and [`Ctx::defer`].
pub trait Platform: Clone + Send + Sync + 'static {
    /// The HTTP client REST calls go through
    type Transport: Transport + Clone + 'static;

    /// A handle to the Transport, for a new client
    fn transport(&self) -> Self::Transport;

    /// Runs `work` after the response is sent (`wait_until` on Cloudflare, `tokio::spawn` on a
    /// hosted server), within the platform's post-response time limit
    fn spawn(&self, work: BackgroundWork);

    /// The bot token (`DISCORD_TOKEN`), without the `Bot ` prefix
    fn bot_token(&self) -> Option<&str>;
}

fn interaction_client<S: Platform>(
    state: &S,
    common: &InteractionCommon,
) -> InteractionClient<S::Transport> {
    InteractionClient::new(
        state.transport(),
        common.application_id,
        common.id,
        common.token.clone(),
    )
}

fn bot_client<S: Platform>(
    state: &S,
    common: &InteractionCommon,
) -> Result<BotClient<S::Transport>, HandlerError> {
    let token = state
        .bot_token()
        .ok_or_else(|| HandlerError::new("the bot token (DISCORD_TOKEN) is not configured"))?;
    Ok(BotClient::new(
        state.transport(),
        token,
        common.application_id,
    ))
}

impl<'a, S: Platform, D: Clone + Send + Sync + 'static> Ctx<'a, S, D> {
    /// Follow-ups and the original response of this Interaction. Before the handler returns,
    /// only Endpoints that do not depend on the initial response work; use [`defer`](Self::defer)
    /// to act after it.
    pub fn client(&self) -> InteractionClient<S::Transport> {
        interaction_client(self.state(), &self.interaction().common)
    }

    /// Client authorized by the bot token; an error when the Adapter has none
    pub fn bot(&self) -> Result<BotClient<S::Transport>, HandlerError> {
        bot_client(self.state(), &self.interaction().common)
    }

    /// Answers with a Deferred Response (the user sees "thinking...") and runs `work` as
    /// Background Work; its output then replaces the original response, like a handler's return
    /// value would have been sent. Wrap in [`Ephemeral`](super::Ephemeral) to make the response
    /// ephemeral; that is decided now, not by `work`'s output.
    ///
    /// ```ignore
    /// #[command]
    /// fn report(ctx: Ctx<Worker>) -> Deferred {
    ///     ctx.defer(async |ctx| {
    ///         let stats = slow_query(ctx.state()).await?;
    ///         Ok::<_, HandlerError>(format!("{stats} rows"))
    ///     })
    /// }
    /// ```
    pub fn defer<F, Fut>(&self, work: F) -> Deferred
    where
        F: FnOnce(DeferredCtx<S, D>) -> Fut,
        Fut: Future<Output: CommandOutput> + Send + 'static,
    {
        self.spawn_completion(work, Target::Original);
        Deferred(())
    }

    /// Runs `work` as Background Work, then edits the original response with its output
    fn spawn_completion<F, Fut>(&self, work: F, on_error: Target)
    where
        F: FnOnce(DeferredCtx<S, D>) -> Fut,
        Fut: Future<Output: CommandOutput> + Send + 'static,
    {
        let theme = self.configured_theme();
        let ctx = DeferredCtx {
            interaction: Arc::clone(self.shared_interaction()),
            state: self.state().clone(),
            theme,
        };
        let client = ctx.client();
        let output = work(ctx);
        // A child of the `interaction` span; entered while spawning so Adapters that instrument
        // `spawn` with the current span pick it up too
        let span = info_span!("background_work");
        let work = async move {
            debug!("background work started");
            let result = complete(client, output, theme, on_error).await;
            #[cfg(feature = "tracing")]
            match &result {
                Ok(()) => debug!("background work done"),
                Err(error) => error.trace("background work failed"),
            }
            result
        }
        .instrument(span.clone());
        span.in_scope(|| self.state().spawn(Box::pin(work)));
    }
}

/// Where Background Work's message goes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    /// An edit of the original response: a deferred reply's loading message, or the component's
    /// message
    Original,
    /// An ephemeral follow-up, leaving the component's message (and its state) as it was; an
    /// error edit would also be rejected on a Components V2 message
    Followup,
}

/// Background Work's body: awaits the handler's output, then edits the original response with
/// it. An error goes where `on_error` says. A failed send, else the handler's error, is the
/// result.
async fn complete<T: Transport>(
    client: InteractionClient<T>,
    output: impl Future<Output: CommandOutput> + Send,
    theme: Option<&'static Theme>,
    on_error: Target,
) -> Result<(), HandlerError> {
    let error = match output.await.into_result().and_then(|response| {
        check_components_v2(&response)?;
        Ok(response)
    }) {
        Ok(response) => return send(&client, response, theme, Target::Original).await,
        Err(error) => error,
    };
    send(&client, error.response_with(theme), theme, on_error).await?;
    Err(error)
}

/// Themes `response` and sends its message as an edit of the original response or a follow-up
async fn send<T: Transport>(
    client: &InteractionClient<T>,
    mut response: InteractionResponse,
    theme: Option<&'static Theme>,
    target: Target,
) -> Result<(), HandlerError> {
    if let Some(theme) = theme {
        theme.apply(&mut response);
    }
    let Some(data) = response.into_message_data() else {
        return Ok(());
    };
    let result = match target {
        Target::Original => {
            // Ephemerality was fixed by the deferred reply; edits cannot change it
            let flags = data
                .flags
                .map(|flags| flags - MessageFlags::Ephemeral)
                .filter(|flags| !flags.is_empty());
            let edit = EditMessage {
                flags,
                ..EditMessage::from(&data)
            };
            until_known(client, || client.edit_original(&edit)).await
        }
        Target::Followup => {
            let followup = CreateFollowup::from(&data);
            until_known(client, || client.create_followup(&followup)).await
        }
    };
    result.map_err(|error| {
        let action = match target {
            Target::Original => "editing the original response",
            Target::Followup => "sending the error follow-up",
        };
        HandlerError::new(format_args!("{action} failed: {error}"))
    })
}

/// Interaction data whose original response is a message that can be updated in place
pub trait Updatable: private::Sealed {}

impl Updatable for MessageComponentData {}
impl Updatable for ModalSubmitData {}

mod private {
    pub trait Sealed {}
    impl Sealed for crate::types::MessageComponentData {}
    impl Sealed for crate::types::ModalSubmitData {}
}

impl<'a, S: Platform, D: Updatable + Clone + Send + Sync + 'static> Ctx<'a, S, D> {
    /// Acknowledges without a loading message (Discord type 6) and runs `work` as Background
    /// Work; its output then replaces the message the component is on. An error leaves the
    /// message as it was and is shown in an ephemeral follow-up instead.
    pub fn defer_update<F, Fut>(&self, work: F) -> DeferredUpdate
    where
        F: FnOnce(DeferredCtx<S, D>) -> Fut,
        Fut: Future<Output: CommandOutput> + Send + 'static,
    {
        self.spawn_completion(work, Target::Followup);
        DeferredUpdate(())
    }
}

/// Retries `request` while Discord has not yet seen the deferred reply: Background Work can
/// start before the platform finishes sending it
async fn until_known<T: Transport, R, Fut: Future<Output = Result<R, Error>>>(
    client: &InteractionClient<T>,
    request: impl Fn() -> Fut,
) -> Result<(), Error> {
    const ATTEMPTS: u32 = 4;
    for attempt in 1.. {
        match request().await {
            Err(Error::Api(error)) if error.status == 404 && attempt < ATTEMPTS => {
                debug!(attempt, "original response not found yet; retrying");
                client
                    .transport()
                    .sleep(Duration::from_millis(250 * u64::from(attempt)))
                    .await;
            }
            result => return result.map(drop),
        }
    }
    unreachable!()
}

/// Returned by [`Ctx::defer`]: a Deferred Response whose Background Work is already scheduled
#[derive(Debug)]
#[must_use = "return it from the handler, or the user sees no response"]
pub struct Deferred(());

impl IntoResponse for Deferred {
    fn into_response(self) -> InteractionResponse {
        InteractionResponse::DeferredChannelMessageWithSource(None)
    }
}

/// Returned by [`Ctx::defer_update`]: an acknowledgement whose Background Work will update the
/// component's message
#[derive(Debug)]
#[must_use = "return it from the handler, or the user sees no response"]
pub struct DeferredUpdate(());

impl IntoResponse for DeferredUpdate {
    fn into_response(self) -> InteractionResponse {
        InteractionResponse::DeferredUpdateMessage
    }
}

/// What deferred Background Work sees: an owned, `'static` copy of the Interaction and State
pub struct DeferredCtx<S, D = ApplicationCommandInteractionData> {
    interaction: Arc<DataInteraction<D>>,
    state: S,
    theme: Option<&'static Theme>,
}

impl<S: Clone, D> Clone for DeferredCtx<S, D> {
    fn clone(&self) -> Self {
        Self {
            interaction: self.interaction.clone(),
            state: self.state.clone(),
            theme: self.theme,
        }
    }
}

impl<S: Platform, D> DeferredCtx<S, D> {
    /// The Interaction being answered
    pub fn interaction(&self) -> &DataInteraction<D> {
        &self.interaction
    }

    /// The Interaction's data
    pub fn data(&self) -> &D {
        &self.interaction.data
    }

    /// Platform resources passed through by the Adapter
    pub fn state(&self) -> &S {
        &self.state
    }

    /// The App's colors and branding; [`Theme::DEFAULT`] when the App has none
    pub fn theme(&self) -> &'static Theme {
        self.theme.unwrap_or(&Theme::DEFAULT)
    }

    /// The invoking user, in guilds and DMs alike
    pub fn user(&self) -> Option<&User> {
        self.interaction.common.invoking_user()
    }

    /// The invoking member, in guilds only
    pub fn member(&self) -> Option<&Member> {
        self.interaction.common.member.as_ref()
    }

    /// The guild it was invoked in; `None` in DMs
    pub fn guild_id(&self) -> Option<Snowflake> {
        self.interaction.common.guild_id
    }

    /// The channel it was invoked in
    pub fn channel_id(&self) -> Option<Snowflake> {
        self.interaction.common.channel_id
    }

    /// Follow-ups and the original response of this Interaction
    pub fn client(&self) -> InteractionClient<S::Transport> {
        interaction_client(&self.state, &self.interaction.common)
    }

    /// Client authorized by the bot token; an error when the Adapter has none
    pub fn bot(&self) -> Result<BotClient<S::Transport>, HandlerError> {
        bot_client(&self.state, &self.interaction.common)
    }

    /// Waits on the platform's timer (counts toward the Background Work time limit)
    pub async fn sleep(&self, duration: Duration) {
        self.state.transport().sleep(duration).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use futures::executor::block_on;
    use serde_json::json;

    use super::*;
    use crate::{
        command::Ephemeral,
        rest::testing::MockTransport,
        types::{ApplicationCommandInteraction, Interaction},
    };

    #[derive(Clone, Default)]
    struct TestPlatform {
        transport: Arc<MockTransport>,
        work: Arc<Mutex<Vec<BackgroundWork>>>,
    }

    impl Platform for TestPlatform {
        type Transport = Arc<MockTransport>;

        fn transport(&self) -> Self::Transport {
            self.transport.clone()
        }

        fn spawn(&self, work: BackgroundWork) {
            self.work.lock().unwrap().push(work);
        }

        fn bot_token(&self) -> Option<&str> {
            None
        }
    }

    impl TestPlatform {
        fn run(&self) -> Result<(), HandlerError> {
            let work = self.work.lock().unwrap().pop().expect("work was spawned");
            block_on(work)
        }

        fn edits(&self) -> Vec<(String, serde_json::Value)> {
            self.transport
                .requests()
                .iter()
                .map(|r| {
                    let body = serde_json::from_slice(r.body.as_deref().unwrap()).unwrap();
                    (r.url.clone(), body)
                })
                .collect()
        }
    }

    fn interaction() -> Arc<ApplicationCommandInteraction> {
        let payload = json!({
            "id": "2", "application_id": "1", "type": 2, "token": "tok", "version": 1,
            "user": { "id": "3", "username": "a", "discriminator": "0", "global_name": null, "avatar": null },
            "data": { "id": "4", "name": "slow", "type": 1 },
        });
        match serde_json::from_value(payload).unwrap() {
            Interaction::ApplicationCommand(interaction) => Arc::new(interaction),
            _ => unreachable!(),
        }
    }

    const ORIGINAL: &str = "https://discord.com/api/v10/webhooks/1/tok/messages/@original";

    #[test]
    fn defer_answers_type_5_and_edits_original_with_the_output() {
        let platform = TestPlatform::default();
        let interaction = interaction();
        let ctx = Ctx::new(&interaction, &platform);
        let response = ctx
            .defer(async |ctx| format!("done for {}", ctx.user().unwrap().username))
            .into_response();
        assert!(matches!(
            response,
            InteractionResponse::DeferredChannelMessageWithSource(None)
        ));
        assert!(platform.transport.requests().is_empty());

        platform.run().unwrap();
        assert_eq!(
            platform.edits(),
            [(ORIGINAL.to_string(), json!({ "content": "done for a" }))]
        );
    }

    #[test]
    fn ephemeral_defer_sets_the_flag() {
        let platform = TestPlatform::default();
        let interaction = interaction();
        let ctx = Ctx::new(&interaction, &platform);
        let response = Ephemeral(ctx.defer(async |_| "ok")).into_response();
        assert!(matches!(
            response,
            InteractionResponse::DeferredChannelMessageWithSource(Some(MessageFlags::Ephemeral))
        ));
    }

    #[test]
    fn errors_edit_with_the_error_message_and_are_reported() {
        let platform = TestPlatform::default();
        let interaction = interaction();
        let ctx = Ctx::new(&interaction, &platform);
        let _ = ctx.defer(async |_| Err::<String, _>(HandlerError::user("nope")));
        assert_eq!(platform.run().unwrap_err().to_string(), "nope");
        assert_eq!(
            platform.edits(),
            [(ORIGINAL.to_string(), json!({ "content": "nope" }))]
        );
    }

    #[test]
    fn retries_while_the_deferral_is_unknown() {
        let transport = MockTransport::default()
            .respond(404, r#"{"code":10015,"message":"Unknown Webhook"}"#)
            .respond(200, "{}");
        let platform = TestPlatform {
            transport: Arc::new(transport),
            ..Default::default()
        };
        let interaction = interaction();
        let _ = Ctx::new(&interaction, &platform).defer(async |_| "ok");
        platform.run().unwrap();
        assert_eq!(platform.edits().len(), 2);
        assert_eq!(platform.transport.sleeps(), [Duration::from_millis(250)]);
    }

    const THEME: Theme = Theme::new().with_primary(0x123456).with_footer("Bot", None);

    #[test]
    fn themed_defer_fills_in_the_edit() {
        let platform = TestPlatform::default();
        let interaction = interaction();
        let ctx = Ctx::new(&interaction, &platform).with_theme(Some(&THEME));
        let _ = ctx.defer(async |ctx| {
            assert_eq!(ctx.theme(), &THEME);
            crate::types::Embed::new().with_title("done")
        });
        platform.run().unwrap();
        let (_, body) = &platform.edits()[0];
        assert_eq!(body["embeds"][0]["color"], 0x123456);
        assert_eq!(body["embeds"][0]["footer"]["text"], "Bot");
    }

    #[test]
    fn themed_defer_fills_container_accents_and_keeps_v2() {
        let platform = TestPlatform::default();
        let interaction = interaction();
        let ctx = Ctx::new(&interaction, &platform).with_theme(Some(&THEME));
        let _ = Ephemeral(ctx.defer(async |_| {
            crate::types::Container::new(vec![crate::types::Component::text("done")])
        }));
        platform.run().unwrap();
        let (_, body) = &platform.edits()[0];
        assert_eq!(body["components"][0]["type"], 17);
        assert_eq!(body["components"][0]["accent_color"], 0x123456);
        assert_eq!(body["flags"], MessageFlags::IsComponentsV2.bits());
        assert_eq!(body.get("content"), None);
    }

    #[test]
    fn themed_defer_errors_are_error_embeds() {
        let platform = TestPlatform::default();
        let interaction = interaction();
        let ctx = Ctx::new(&interaction, &platform).with_theme(Some(&THEME));
        let _ = ctx.defer(async |_| Err::<String, _>(HandlerError::user("nope")));
        assert!(platform.run().is_err());
        let (_, body) = &platform.edits()[0];
        assert_eq!(body["embeds"][0]["color"], Theme::DEFAULT.error);
        assert_eq!(body["embeds"][0]["description"], "nope");
        assert_eq!(body.get("flags"), None);
    }

    fn component() -> Arc<crate::types::MessageComponentInteraction> {
        let payload = json!({
            "id": "2", "application_id": "1", "type": 3, "token": "tok", "version": 1,
            "user": { "id": "3", "username": "a", "discriminator": "0", "global_name": null, "avatar": null },
            "data": { "custom_id": "roll", "component_type": 2 },
        });
        match serde_json::from_value(payload).unwrap() {
            Interaction::MessageComponent(interaction) => Arc::new(interaction),
            _ => unreachable!(),
        }
    }

    const FOLLOWUP: &str = "https://discord.com/api/v10/webhooks/1/tok";

    #[test]
    fn defer_update_errors_are_ephemeral_followups() {
        let platform = TestPlatform::default();
        let interaction = component();
        let ctx = Ctx::new(&interaction, &platform);
        let _ = ctx.defer_update(async |_| Err::<String, _>(HandlerError::user("nope")));
        assert_eq!(platform.run().unwrap_err().to_string(), "nope");
        assert_eq!(
            platform.edits(),
            [(
                FOLLOWUP.to_string(),
                json!({ "content": "nope", "flags": MessageFlags::Ephemeral.bits() })
            )]
        );
    }

    #[test]
    fn defer_update_output_edits_the_message() {
        let platform = TestPlatform::default();
        let interaction = component();
        let _ = Ctx::new(&interaction, &platform).defer_update(async |_| "rolled");
        platform.run().unwrap();
        assert_eq!(
            platform.edits(),
            [(ORIGINAL.to_string(), json!({ "content": "rolled" }))]
        );
    }

    #[test]
    fn components_v2_output_with_content_is_an_error() {
        let platform = TestPlatform::default();
        let interaction = interaction();
        let _ = Ctx::new(&interaction, &platform).defer(async |_| {
            let mut response =
                InteractionResponse::respond_with_components(vec![crate::types::Component::text(
                    "hi",
                )]);
            response.message_data_mut().unwrap().content = Some("hi".into());
            response
        });
        let error = platform.run().unwrap_err();
        assert!(error.to_string().contains("Components V2"), "{error}");
        assert_eq!(
            platform.edits(),
            [(
                ORIGINAL.to_string(),
                json!({ "content": "Something went wrong handling this interaction." })
            )]
        );
    }

    #[test]
    fn bot_needs_a_token() {
        let platform = TestPlatform::default();
        let interaction = interaction();
        assert!(Ctx::new(&interaction, &platform).bot().is_err());
    }
}
