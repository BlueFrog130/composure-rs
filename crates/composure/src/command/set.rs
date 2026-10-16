use crate::{
    definition::{
        ApplicationCommand, ApplicationCommandOption, ChatInputCommand, CommandDetails,
        SubcommandCommandOption, SubcommandGroupOption, SubcommandOption,
    },
    types::{
        ApplicationCommandInteraction, ApplicationCommandInteractionData,
        ApplicationCommandInteractionDataOption as DataOption, ApplicationCommandType,
        ApplicationIntegrationType, InteractionContextType, InteractionResponse,
        MessageComponentInteraction, ModalSubmitInteraction, OptionList, Permissions, Subcommand,
        TypeField,
    },
};

use super::{Ctx, HandlerError};

/// What Discord stores for a top-level Command. Implemented by `#[command]` and `#[group]`.
pub trait CommandDefinition {
    const NAME: &'static str;
    const KIND: ApplicationCommandType;

    /// Named Scope the Command is synced to; `None` is global
    const SCOPE: Option<&'static str> = None;

    fn definition() -> ApplicationCommand;
}

/// A chat-input Command or Group that can be nested in a Group.
pub trait SubcommandDefinition: CommandDefinition {
    /// 0 for a Command, 1 for a Group of Commands, 2 for a Group containing Groups
    const DEPTH: u8;

    /// This Command as a subcommand, or this Group as a subcommand group
    fn option_definition() -> ApplicationCommandOption;
}

/// Runs a Command for an Interaction. Implemented by `#[command]` and `#[group]`; Groups route to
/// the subcommand that was invoked.
#[allow(async_fn_in_trait)] // dispatch is static and Workers futures are not Send
pub trait Command<S>: CommandDefinition {
    async fn invoke(invocation: Invocation<'_, S>) -> Result<InteractionResponse, HandlerError>;
}

static NO_OPTIONS: OptionList = OptionList::empty();

/// One Command (or subcommand) being invoked: the Interaction, the State, and the Options at
/// this level of the subcommand tree.
pub struct Invocation<'a, S> {
    interaction: &'a ApplicationCommandInteraction,
    state: &'a S,
    options: &'a OptionList,
    /// Set when routed into a subcommand group: the subcommand chosen inside it
    subcommand: Option<&'a Subcommand>,
}

impl<'a, S> Invocation<'a, S> {
    pub fn new(interaction: &'a ApplicationCommandInteraction, state: &'a S) -> Self {
        Self {
            interaction,
            state,
            options: interaction.data.options.as_ref().unwrap_or(&NO_OPTIONS),
            subcommand: None,
        }
    }

    pub fn ctx(&self) -> Ctx<'a, S> {
        Ctx::new(self.interaction, self.state)
    }

    pub fn data(&self) -> &'a ApplicationCommandInteractionData {
        &self.interaction.data
    }

    /// The Option named `name` at this level
    pub fn option(&self, name: &str) -> Option<&'a DataOption> {
        self.options.get_option(name)
    }

    /// The subcommand or subcommand group invoked below this Group, and the Invocation for it
    pub fn route(&self) -> Option<(&'a str, Invocation<'a, S>)> {
        if let Some(subcommand) = self.subcommand {
            return Some((&subcommand.name, self.below(&subcommand.options, None)));
        }
        if let Some(subcommand) = self.options.subcommand() {
            return Some((&subcommand.name, self.below(&subcommand.options, None)));
        }
        self.options.subcommand_group().map(|group| {
            (
                group.name.as_str(),
                self.below(&NO_OPTIONS, Some(&group.subcommand)),
            )
        })
    }

    fn below(&self, options: &'a OptionList, subcommand: Option<&'a Subcommand>) -> Self {
        Self {
            interaction: self.interaction,
            state: self.state,
            options,
            subcommand,
        }
    }
}

/// Name, description, and Discord-side settings of a Command or Group, from its attribute
#[derive(Debug, Clone, Copy)]
pub struct CommandMeta {
    pub name: &'static str,
    pub description: &'static str,
    pub default_member_permissions: Option<Permissions>,
    pub integration_types: Option<&'static [ApplicationIntegrationType]>,
    pub contexts: Option<&'static [InteractionContextType]>,
    pub nsfw: Option<bool>,
}

impl CommandMeta {
    pub const fn new(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            description,
            default_member_permissions: None,
            integration_types: None,
            contexts: None,
            nsfw: None,
        }
    }

    fn details<const T: u8>(&self) -> CommandDetails<T> {
        CommandDetails {
            t: TypeField,
            id: None,
            application_id: None,
            guild_id: None,
            name: self.name.to_string(),
            name_localizations: None,
            default_member_permissions: self.default_member_permissions,
            dm_permission: None,
            integration_types: self.integration_types.map(<[_]>::to_vec),
            contexts: self.contexts.map(<[_]>::to_vec),
            nsfw: self.nsfw,
            version: None,
        }
    }

    /// A chat-input Command, or a Group whose `options` are its subcommands
    pub fn chat_input(&self, options: Vec<ApplicationCommandOption>) -> ApplicationCommand {
        ApplicationCommand::ChatInputCommand(ChatInputCommand {
            details: self.details(),
            description: self.description.to_string(),
            description_localizations: None,
            options: (!options.is_empty()).then_some(options),
        })
    }

    pub fn user(&self) -> ApplicationCommand {
        ApplicationCommand::UserCommand(self.details())
    }

    pub fn message(&self) -> ApplicationCommand {
        ApplicationCommand::MessageCommand(self.details())
    }

    /// This Command nested in a Group
    pub fn subcommand(&self, options: Vec<ApplicationCommandOption>) -> ApplicationCommandOption {
        let options: Vec<SubcommandCommandOption> = options
            .into_iter()
            .filter_map(|option| option.try_into().ok())
            .collect();
        ApplicationCommandOption::Subcommand(SubcommandOption {
            t: TypeField,
            name: self.name.to_string(),
            name_localizations: None,
            description: self.description.to_string(),
            description_localizations: None,
            options: (!options.is_empty()).then_some(options),
        })
    }

    /// This Group nested in another Group; `children` are subcommand definitions
    pub fn subcommand_group(
        &self,
        children: Vec<ApplicationCommandOption>,
    ) -> ApplicationCommandOption {
        let options = children
            .into_iter()
            .filter_map(|child| match child {
                ApplicationCommandOption::Subcommand(subcommand) => Some(subcommand),
                _ => None,
            })
            .collect();
        ApplicationCommandOption::SubcommandGroup(SubcommandGroupOption {
            t: TypeField,
            name: self.name.to_string(),
            name_localizations: None,
            description: self.description.to_string(),
            description_localizations: None,
            options: Some(options),
        })
    }
}

/// A Command Definition and the Scope it syncs to
#[derive(Debug)]
pub struct ScopedDefinition {
    pub scope: Option<&'static str>,
    pub command: ApplicationCommand,
}

impl ScopedDefinition {
    pub fn of<C: CommandDefinition>() -> Self {
        Self {
            scope: C::SCOPE,
            command: C::definition(),
        }
    }
}

/// The Commands a bot serves. Implemented by [`commands!`](crate::commands).
#[allow(async_fn_in_trait)]
pub trait CommandSet<S> {
    /// Runs the matching Command, or `None` if no Command has the invoked name and type
    async fn dispatch(
        &self,
        interaction: &ApplicationCommandInteraction,
        state: &S,
    ) -> Option<Result<InteractionResponse, HandlerError>>;

    /// Runs the Component Handler whose prefix the `custom_id` starts with, or `None`
    async fn dispatch_component(
        &self,
        _interaction: &MessageComponentInteraction,
        _state: &S,
    ) -> Option<Result<InteractionResponse, HandlerError>> {
        None
    }

    /// Runs the Modal Handler whose prefix the `custom_id` starts with, or `None`
    async fn dispatch_modal(
        &self,
        _interaction: &ModalSubmitInteraction,
        _state: &S,
    ) -> Option<Result<InteractionResponse, HandlerError>> {
        None
    }
}

/// The Command Definitions of a Command Set, for Sync
pub trait CommandSetDefinition {
    fn definitions() -> Vec<ScopedDefinition>;
}

/// Compile-time checks for the Commands of a Command Set or Group, as `(name, kind, scope)`: no
/// two share a name and kind, and no Scope exceeds Discord's per-kind limit. Used by
/// [`commands!`](crate::commands) and `#[group]`.
#[doc(hidden)]
pub const fn check_commands(commands: &[(&str, ApplicationCommandType, Option<&str>)]) {
    let mut i = 0;
    while i < commands.len() {
        let (name, kind, scope) = commands[i];
        let limit = match kind {
            ApplicationCommandType::ChatInput => 100,
            ApplicationCommandType::User | ApplicationCommandType::Message => 15,
        };
        let mut same_scope = 0;
        let mut j = 0;
        while j < commands.len() {
            let (other_name, other_kind, other_scope) = commands[j];
            if kind as u8 == other_kind as u8 {
                if j != i && str_eq(name, other_name) {
                    panic!("two commands have the same name and type");
                }
                if scope_eq(scope, other_scope) {
                    same_scope += 1;
                }
            }
            j += 1;
        }
        if same_scope > limit {
            panic!(
                "too many commands of one type in one scope: Discord allows 100 chat-input, 15 user, and 15 message commands"
            );
        }
        i += 1;
    }
}

pub(crate) const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn scope_eq(a: Option<&str>, b: Option<&str>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => str_eq(a, b),
        (None, None) => true,
        _ => false,
    }
}
