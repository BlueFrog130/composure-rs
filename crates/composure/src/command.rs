//! Commands defined once with `#[command]`, from which both their handler and their Command
//! Definition derive, collected into a Command Set with [`commands!`](crate::commands).
//!
//! ```ignore
//! use composure::{command, commands};
//!
//! /// Repeat some text
//! #[command]
//! fn echo(
//!     #[option("What to say", max_len = 200)] text: &str,
//!     #[option("How many times", min = 1, max = 5)] times: Option<u8>,
//! ) -> String {
//!     text.repeat(times.unwrap_or(1).into())
//! }
//!
//! commands! {
//!     pub struct Bot = [echo];
//! }
//! ```

mod autocomplete;
mod component;
mod ctx;
pub mod custom_id;
mod error;
mod event;
mod handler;
mod option;
#[cfg(feature = "rest")]
mod platform;
mod response;
mod set;

pub use autocomplete::*;
pub use component::*;
pub use ctx::*;
pub use error::*;
pub use event::*;
pub use handler::*;
pub use option::*;
#[cfg(feature = "rest")]
pub use platform::*;
pub use response::*;
pub use set::*;

/// Declares a Command Set: a unit struct that dispatches to the listed Commands and Groups (and
/// optionally Component and Modal Handlers) and yields their Definitions for Sync.
///
/// ```ignore
/// commands! {
///     /// Everything the bot serves
///     pub struct Bot = [ping, echo, Admin];
///     components = [counter_click];
///     modals = [on_feedback];
///     events = MyEvents;
/// }
/// ```
///
/// The struct implements [`InteractionHandler`] for every State its Commands accept; an
/// Interaction for a Command not in the set is answered with an "Unknown command" message.
/// Autocomplete goes to the invoked Command's Autocomplete Handler for the focused Option.
/// Webhook Events go to `events` (a unit struct implementing [`EventHandler`]), or are ignored.
/// Components and modals are routed by the `custom_id` prefix; unmatched ones get a "not
/// handled" message. Two Commands with the same name and type, two handlers with the same
/// prefix, or more Commands than Discord allows in one Scope fail to compile.
/// `Bot.with_fallback(handler)` sends components, modals, autocomplete, and unknown commands to
/// `handler` instead.
#[macro_export]
macro_rules! commands {
    (
        $(#[$attr:meta])* $vis:vis struct $name:ident = [$($command:path),* $(,)?];
        $(components = [$($component:path),* $(,)?];)?
        $(modals = [$($modal:path),* $(,)?];)?
        $(events = $events:path;)?
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, Default)]
        $vis struct $name;

        const _: () = $crate::command::check_commands(&[$((
            <$command as $crate::command::CommandDefinition>::NAME,
            <$command as $crate::command::CommandDefinition>::KIND,
            <$command as $crate::command::CommandDefinition>::SCOPE,
        )),*]);

        const _: () = $crate::command::check_prefixes(&[$($(
            <$component as $crate::command::ComponentDefinition>::PREFIX,
        )*)?]);
        const _: () = $crate::command::check_prefixes(&[$($(
            <$modal as $crate::command::ModalDefinition>::PREFIX,
        )*)?]);

        impl $name {
            /// Sends components, modals, autocomplete, and commands not in this set to `fallback`
            pub fn with_fallback<F>(self, fallback: F) -> $crate::command::WithFallback<Self, F> {
                $crate::command::WithFallback::new(self, fallback)
            }
        }

        impl<S> $crate::command::CommandSet<S> for $name
        where
            $($command: $crate::command::Command<S>,)*
            $($($component: $crate::command::ComponentHandler<S>,)*)?
            $($($modal: $crate::command::ModalHandler<S>,)*)?
        {
            async fn dispatch(
                &self,
                interaction: &$crate::types::ApplicationCommandInteraction,
                state: &S,
            ) -> ::core::option::Option<
                ::core::result::Result<$crate::types::InteractionResponse, $crate::command::HandlerError>,
            > {
                let data = &interaction.data;
                $(
                    if data.name == <$command as $crate::command::CommandDefinition>::NAME
                        && data.t == <$command as $crate::command::CommandDefinition>::KIND
                    {
                        return ::core::option::Option::Some(
                            <$command as $crate::command::Command<S>>::invoke(
                                $crate::command::Invocation::new(interaction, state),
                            )
                            .await,
                        );
                    }
                )*
                ::core::option::Option::None
            }

            async fn dispatch_autocomplete(
                &self,
                interaction: &$crate::types::ApplicationCommandInteraction,
                state: &S,
            ) -> ::core::option::Option<
                ::core::result::Result<$crate::types::InteractionResponse, $crate::command::HandlerError>,
            > {
                let data = &interaction.data;
                $(
                    if data.name == <$command as $crate::command::CommandDefinition>::NAME
                        && data.t == <$command as $crate::command::CommandDefinition>::KIND
                    {
                        return <$command as $crate::command::Command<S>>::autocomplete(
                            $crate::command::Invocation::new(interaction, state),
                        )
                        .await;
                    }
                )*
                ::core::option::Option::None
            }

            #[allow(unused_variables)]
            async fn dispatch_component(
                &self,
                interaction: &$crate::types::MessageComponentInteraction,
                state: &S,
            ) -> ::core::option::Option<
                ::core::result::Result<$crate::types::InteractionResponse, $crate::command::HandlerError>,
            > {
                let (prefix, fields) = $crate::command::custom_id::split(&interaction.data.custom_id);
                $($(
                    if prefix == <$component as $crate::command::ComponentDefinition>::PREFIX {
                        return ::core::option::Option::Some(
                            <$component as $crate::command::ComponentHandler<S>>::invoke(
                                interaction, state, fields,
                            )
                            .await,
                        );
                    }
                )*)?
                ::core::option::Option::None
            }

            #[allow(unused_variables)]
            async fn dispatch_modal(
                &self,
                interaction: &$crate::types::ModalSubmitInteraction,
                state: &S,
            ) -> ::core::option::Option<
                ::core::result::Result<$crate::types::InteractionResponse, $crate::command::HandlerError>,
            > {
                let (prefix, fields) = $crate::command::custom_id::split(&interaction.data.custom_id);
                $($(
                    if prefix == <$modal as $crate::command::ModalDefinition>::PREFIX {
                        return ::core::option::Option::Some(
                            <$modal as $crate::command::ModalHandler<S>>::invoke(
                                interaction, state, fields,
                            )
                            .await,
                        );
                    }
                )*)?
                ::core::option::Option::None
            }
        }

        impl<S> $crate::command::InteractionHandler<S> for $name
        where
            $name: $crate::command::CommandSet<S>,
        {
            async fn command(
                &self,
                command: $crate::types::ApplicationCommandInteraction,
                state: &S,
            ) -> ::core::result::Result<$crate::types::InteractionResponse, $crate::command::HandlerError> {
                match $crate::command::CommandSet::dispatch(self, &command, state).await {
                    ::core::option::Option::Some(result) => result,
                    ::core::option::Option::None => ::core::result::Result::Err(
                        $crate::command::HandlerError::unknown_command(&command.data.name),
                    ),
                }
            }

            async fn autocomplete(
                &self,
                autocomplete: $crate::types::ApplicationCommandInteraction,
                state: &S,
            ) -> ::core::result::Result<$crate::types::InteractionResponse, $crate::command::HandlerError> {
                match $crate::command::CommandSet::dispatch_autocomplete(self, &autocomplete, state).await {
                    ::core::option::Option::Some(result) => result,
                    ::core::option::Option::None => {
                        $crate::command::InteractionHandler::<S>::autocomplete(
                            &$crate::command::NoHandler, autocomplete, state,
                        )
                        .await
                    }
                }
            }

            async fn component(
                &self,
                component: $crate::types::MessageComponentInteraction,
                state: &S,
            ) -> ::core::result::Result<$crate::types::InteractionResponse, $crate::command::HandlerError> {
                match $crate::command::CommandSet::dispatch_component(self, &component, state).await {
                    ::core::option::Option::Some(result) => result,
                    ::core::option::Option::None => {
                        $crate::command::InteractionHandler::<S>::component(
                            &$crate::command::NoHandler, component, state,
                        )
                        .await
                    }
                }
            }

            async fn modal_submit(
                &self,
                modal: $crate::types::ModalSubmitInteraction,
                state: &S,
            ) -> ::core::result::Result<$crate::types::InteractionResponse, $crate::command::HandlerError> {
                match $crate::command::CommandSet::dispatch_modal(self, &modal, state).await {
                    ::core::option::Option::Some(result) => result,
                    ::core::option::Option::None => {
                        $crate::command::InteractionHandler::<S>::modal_submit(
                            &$crate::command::NoHandler, modal, state,
                        )
                        .await
                    }
                }
            }
        }

        $crate::__command_set_events!($name $(, $events)?);

        impl $crate::command::CommandSetDefinition for $name {
            fn definitions() -> ::std::vec::Vec<$crate::command::ScopedDefinition> {
                ::std::vec![$($crate::command::ScopedDefinition::of::<$command>()),*]
            }
        }
    };
}
