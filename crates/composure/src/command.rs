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

mod ctx;
mod error;
mod handler;
mod option;
#[cfg(feature = "rest")]
mod platform;
mod response;
mod set;

pub use ctx::*;
pub use error::*;
pub use handler::*;
pub use option::*;
#[cfg(feature = "rest")]
pub use platform::*;
pub use response::*;
pub use set::*;

/// Declares a Command Set: a unit struct that dispatches to the listed Commands and Groups and
/// yields their Definitions for Sync.
///
/// ```ignore
/// commands! {
///     /// Everything the bot serves
///     pub struct Bot = [ping, echo, Admin];
/// }
/// ```
///
/// The struct implements [`InteractionHandler`] for every State its Commands accept; an
/// Interaction for a Command not in the set is answered with an "Unknown command" message.
/// Two Commands with the same name and type, or more than Discord allows in one Scope, fail to
/// compile.
/// `Bot.with_fallback(handler)` sends components, modals, autocomplete, and unknown commands to
/// `handler` instead.
#[macro_export]
macro_rules! commands {
    ($(#[$attr:meta])* $vis:vis struct $name:ident = [$($command:path),* $(,)?];) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Copy, Default)]
        $vis struct $name;

        const _: () = $crate::command::check_commands(&[$((
            <$command as $crate::command::CommandDefinition>::NAME,
            <$command as $crate::command::CommandDefinition>::KIND,
            <$command as $crate::command::CommandDefinition>::SCOPE,
        )),*]);

        impl $name {
            /// Sends components, modals, autocomplete, and commands not in this set to `fallback`
            pub fn with_fallback<F>(self, fallback: F) -> $crate::command::WithFallback<Self, F> {
                $crate::command::WithFallback::new(self, fallback)
            }
        }

        impl<S> $crate::command::CommandSet<S> for $name
        where
            $($command: $crate::command::Command<S>,)*
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
        }

        impl $crate::command::CommandSetDefinition for $name {
            fn definitions() -> ::std::vec::Vec<$crate::command::ScopedDefinition> {
                ::std::vec![$($crate::command::ScopedDefinition::of::<$command>()),*]
            }
        }
    };
}
