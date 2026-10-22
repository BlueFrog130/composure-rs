//! Testing bots without Discord or a platform (`testing` feature): build Interactions, run them
//! through an App with a stand-in [`Platform`](crate::command::Platform), and check the response
//! and the REST requests it made.
//!
//! ```
//! # #[cfg(feature = "macros")] mod example {
//! use composure::{app, command};
//!
//! /// Repeat some text
//! #[command]
//! fn echo(#[option("What")] text: &str) -> String {
//!     text.to_uppercase()
//! }
//!
//! #[app(handlers(echo))]
//! pub struct Bot;
//! # }
//! # #[cfg(feature = "macros")] fn main() {
//! # use example::Bot;
//! use composure::testing::{self, ResponseExt, TestPlatform};
//!
//! let response = testing::answer(&Bot, testing::command("echo").option("text", "hi"), &TestPlatform::new());
//! assert_eq!(response.text().as_deref(), Some("HI"));
//! # }
//! # #[cfg(not(feature = "macros"))] fn main() {}
//! ```
//!
//! | | |
//! |---|---|
//! | [`command`], [`user_command`], [`message_command`], [`autocomplete`], [`component`], [`modal`] | Interactions, invoked by [`users::alice`] in a DM unless told otherwise ([`InteractionBuilder`]) |
//! | [`TestPlatform`] | A State with a [`MockTransport`], Bot Data, a bot token, and Background Work run on demand |
//! | [`run`], [`answer`] | Dispatch to an App, as an Adapter does |
//! | [`ResponseExt`] | Reading responses: text, components, buttons, embeds |
//! | [`TestKey`] | Signing requests for [`adapter::handle_interaction`](crate::adapter::handle_interaction) and [`handle_event`](crate::adapter::handle_event) |
//!
//! See [Testing](https://bluefrog130.github.io/composure-rs/testing.html).

use std::{
    future::Future,
    pin::pin,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread::{self, Thread},
};

use crate::{
    command::{HandlerError, InteractionHandler, dispatch},
    types::{Interaction, InteractionResponse, Snowflake},
};

mod builders;
mod platform;
mod response;
mod sign;
mod transport;

pub use builders::{
    CommandData, ComponentData, InteractionBuilder, ModalData, OptionValue, autocomplete, command,
    component, message_command, modal, user_command,
};
pub use platform::TestPlatform;
pub use response::ResponseExt;
pub use sign::{SignedRequest, TestKey};
pub use transport::{MockTransport, json_response};

/// The application id of built Interactions
pub const APPLICATION_ID: Snowflake = Snowflake::from_u64(1052322265397739523);
/// The Interaction token of built Interactions: the Interaction Endpoints' paths contain it
/// (`/webhooks/{APPLICATION_ID}/token/messages/@original`)
pub const INTERACTION_TOKEN: &str = "token";
/// The channel built Interactions happen in, unless set with [`InteractionBuilder::channel`]
pub const CHANNEL_ID: Snowflake = Snowflake::from_u64(300000000000000001);
/// A guild id for [`InteractionBuilder::in_guild`]
pub const GUILD_ID: Snowflake = Snowflake::from_u64(798662131062931547);
/// The id of the message [`message_command`] targets
pub const MESSAGE_ID: Snowflake = Snowflake::from_u64(500000000000000001);

/// Users to invoke Interactions as, and to pass as Options
pub mod users {
    use crate::types::{Snowflake, User};

    /// Alice (`100000000000000001`), who invokes built Interactions unless told otherwise
    pub fn alice() -> User {
        user(100000000000000001, "alice", Some("Alice"))
    }

    /// Bob (`100000000000000002`)
    pub fn bob() -> User {
        user(100000000000000002, "bob", Some("Bob"))
    }

    /// A user on the unique-username system, without an avatar
    pub fn user(id: u64, username: &str, global_name: Option<&str>) -> User {
        User {
            avatar: None,
            discriminator: "0".into(),
            global_name: global_name.map(Into::into),
            id: Snowflake::from_u64(id),
            public_flags: 0,
            username: username.into(),
        }
    }
}

/// Runs `future` to completion on the current thread: enough for handlers, which don't depend
/// on a runtime, without an executor dependency. Allocates nothing itself, so allocation counts
/// measure the future alone.
pub fn block_on<F: Future>(future: F) -> F::Output {
    struct Unpark(Thread);

    impl Wake for Unpark {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }

    thread_local! {
        static WAKER: Waker = Waker::from(Arc::new(Unpark(thread::current())));
    }

    let waker = WAKER.with(Waker::clone);
    let mut cx = Context::from_waker(&waker);
    let mut future = pin!(future);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(output) => return output,
            Poll::Pending => thread::park(),
        }
    }
}

/// Dispatches `interaction` to `handler` with `state`, as an Adapter does: routing, the
/// handler. The handler's error is returned as is; [`answer`] renders it.
pub fn run<S, H: InteractionHandler<S>>(
    handler: &H,
    interaction: impl Into<Interaction>,
    state: &S,
) -> Result<InteractionResponse, HandlerError> {
    block_on(dispatch(interaction.into(), handler, state))
}

/// [`run`], answering an error with the ephemeral message an Adapter sends instead
/// ([`InteractionHandler::render_error`])
pub fn answer<S, H: InteractionHandler<S>>(
    handler: &H,
    interaction: impl Into<Interaction>,
    state: &S,
) -> InteractionResponse {
    run(handler, interaction, state).unwrap_or_else(|error| handler.render_error(&error))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        command::{HasData, Platform},
        rest::{Method, Request, Transport},
        types::{ApplicationCommandInteraction, Interaction, Permissions},
    };

    /// Answers `/fail` with an error, `/later` deferred (Background Work then GETs `/later`), and
    /// anything else with its name
    struct Echo;

    impl<S: Platform> InteractionHandler<S> for Echo {
        async fn command(
            &self,
            command: ApplicationCommandInteraction,
            state: &S,
        ) -> Result<InteractionResponse, HandlerError> {
            match command.data.name.as_str() {
                "fail" => Err(HandlerError::user("nope")),
                "later" => {
                    let transport = state.transport();
                    state.spawn(Box::pin(async move {
                        let request = Request {
                            method: Method::Get,
                            url: format!("{}/later", crate::rest::DISCORD_API),
                            headers: Vec::new(),
                            body: None,
                        };
                        transport.send(&request).await.map_err(HandlerError::new)?;
                        Ok(())
                    }));
                    Ok(InteractionResponse::DeferredChannelMessageWithSource(None))
                }
                name => Ok(InteractionResponse::respond_with_message(name.to_string())),
            }
        }
    }

    #[test]
    fn builds_commands_as_discord_sends_them() {
        let json = command("mod")
            .group("users")
            .subcommand("ban")
            .user_option("user", &users::bob())
            .option("days", 7)
            .option("reason", "spam")
            .in_guild(GUILD_ID)
            .permissions(Permissions::BanMembers)
            .locale("fr")
            .to_json();
        assert_eq!(json["type"], 2);
        assert_eq!(json["guild_id"], "798662131062931547");
        assert_eq!(json["member"]["user"]["username"], "alice");
        assert_eq!(json["member"]["permissions"], "4");
        assert_eq!(json.get("user"), None);
        assert_eq!(json["locale"], "fr");
        let group = &json["data"]["options"][0];
        assert_eq!(
            (&group["name"], &group["type"]),
            (&json!("users"), &json!(2))
        );
        let ban = &group["options"][0];
        assert_eq!((&ban["name"], &ban["type"]), (&json!("ban"), &json!(1)));
        assert_eq!(
            ban["options"],
            json!([
                { "name": "user", "type": 6, "value": "100000000000000002" },
                { "name": "days", "type": 4, "value": 7 },
                { "name": "reason", "type": 3, "value": "spam" },
            ])
        );
        assert_eq!(
            json["data"]["resolved"]["users"]["100000000000000002"]["username"],
            "bob"
        );

        // Parses as an Adapter parses it
        let Interaction::ApplicationCommand(command) = command("ping").by(&users::bob()).build()
        else {
            panic!("a command")
        };
        assert_eq!(command.data.name, "ping");
        assert_eq!(command.common.invoking_user().unwrap().username, "bob");
    }

    #[test]
    fn builds_every_interaction_kind() {
        assert!(matches!(
            user_command("Avatar", &users::bob()).build(),
            Interaction::ApplicationCommand(_)
        ));
        let Interaction::ApplicationCommand(message) = message_command("Quote", "hello").build()
        else {
            panic!("a command")
        };
        assert_eq!(message.data.target_id, Some(MESSAGE_ID));
        assert!(matches!(
            autocomplete("fruit").focused("name", "Bl").build(),
            Interaction::ApplicationCommandAutocomplete(_)
        ));

        let select = component("colors").values(["red", "blue"]).to_json();
        assert_eq!(
            select["data"],
            json!({ "custom_id": "colors", "component_type": 3, "values": ["red", "blue"] })
        );
        assert!(matches!(
            component("counter:4").build(),
            Interaction::MessageComponent(_)
        ));

        let submit = modal("feedback")
            .field("topic", "Docs")
            .select("kind", ["Idea"]);
        let json = submit.to_json();
        assert_eq!(
            json["data"]["components"][1]["component"]["values"],
            json!(["Idea"])
        );
        assert!(matches!(
            modal("feedback").field("topic", "Docs").build(),
            Interaction::ModalSubmit(_)
        ));

        let edited = command("ping").json(|json| json["guild_locale"] = "de".into());
        assert_eq!(edited.to_json()["guild_locale"], "de");
    }

    #[test]
    fn runs_and_answers_like_an_adapter() {
        let platform = TestPlatform::new();
        let response = answer(&Echo, command("echo").option("text", "hi"), &platform);
        assert_eq!(response.response_type(), 4);
        assert_eq!(response.text().as_deref(), Some("echo"));
        assert!(!response.is_ephemeral());

        assert_eq!(
            run(&Echo, command("fail"), &platform)
                .unwrap_err()
                .to_string(),
            "nope"
        );
        let error = answer(&Echo, command("fail"), &platform);
        assert_eq!(error.text().as_deref(), Some("nope"));
        assert!(error.is_ephemeral());
    }

    #[test]
    fn background_work_runs_on_demand() {
        let platform = TestPlatform::new();
        let response = answer(&Echo, command("later"), &platform);
        assert_eq!(response.response_type(), 5);
        assert_eq!(platform.queued_work(), 1);
        assert!(platform.transport().requests().is_empty());

        platform.run_background().unwrap();
        assert_eq!(platform.queued_work(), 0);
        assert_eq!(
            platform.transport().sent(),
            [(Method::Get, "/later".into())]
        );
    }

    #[test]
    fn platform_data_and_token() {
        let platform = TestPlatform::new().with_data(41).with_bot_token(None);
        assert_eq!(*platform.data(), 41);
        assert_eq!(Platform::bot_token(&platform), None);
        assert_eq!(Platform::bot_token(&TestPlatform::new()), Some("bot-token"));
    }

    #[test]
    fn transport_queues_then_answers_then_defaults() {
        let transport = MockTransport::new()
            .respond(404, "{}")
            .answering(|request| {
                request
                    .url
                    .starts_with("https://example.com")
                    .then(|| json_response(200, &json!({ "from": "example" })))
            });
        let request = |url: &str| crate::rest::Request {
            method: Method::Get,
            url: url.into(),
            headers: Vec::new(),
            body: None,
        };
        let send = |url: &str| block_on(transport.send(&request(url))).unwrap();
        assert_eq!(send("https://example.com/a").status, 404, "queued first");
        assert_eq!(send("https://example.com/a").body, br#"{"from":"example"}"#);
        assert_eq!(send("https://discord.com/api/v10/users/@me").body, b"{}");
        assert_eq!(
            transport.paths(),
            [
                "https://example.com/a",
                "https://example.com/a",
                "/users/@me"
            ]
        );
    }

    #[test]
    fn responses_find_components() {
        let response = json!({
            "type": 4,
            "data": { "flags": 32768, "components": [{ "type": 17, "components": [
                { "type": 10, "content": "one" },
                { "type": 9, "components": [{ "type": 10, "content": "two" }],
                  "accessory": { "type": 2, "style": 1, "label": "Next", "custom_id": "page:2" } },
            ] }] },
        });
        assert_eq!(response.text().as_deref(), Some("one\ntwo"));
        assert_eq!(response.custom_id("Next").as_deref(), Some("page:2"));
        assert_eq!(response.button("Back"), None);
        assert_eq!(response.components().len(), 5);
    }

    #[test]
    fn keys_sign_like_discord() {
        let key = TestKey::default();
        let request = key.signed(command("ping").body());
        let public_key = block_on(crate::auth::PublicKey::from_hex(&key.public_key())).unwrap();
        assert!(
            block_on(public_key.verify(&request.signature, &request.timestamp, &request.body))
                .is_ok()
        );
        assert_eq!(request.headers()[0].0, crate::adapter::SIGNATURE_HEADER);
    }
}
