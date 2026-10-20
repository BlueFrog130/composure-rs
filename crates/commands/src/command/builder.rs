use composure::types::{
    ApplicationIntegrationType, InteractionContextType, Permissions, Snowflake, TypeField,
};

use crate::command::*;

/// Builds the chat-input Command Definitions of an application, one closure per Command
pub struct CommandsBuilder {
    /// The application the Commands belong to
    pub application_id: Snowflake,
    /// The guild, for guild Commands; `None` for global ones
    pub guild_id: Option<Snowflake>,
    /// The Commands built so far
    pub commands: Vec<ApplicationCommand>,
}

impl CommandsBuilder {
    /// No Commands yet
    pub fn new(application_id: Snowflake, guild_id: Option<Snowflake>) -> Self {
        Self {
            commands: Vec::new(),
            application_id,
            guild_id,
        }
    }

    /// Adds a Command, built by `command_builder` from an empty [`CommandBuilder`]
    pub fn add_command<F>(mut self, command_builder: F) -> Self
    where
        F: FnOnce(CommandBuilder) -> CommandBuilder,
    {
        let command = command_builder(CommandBuilder::new()).build_chat_command();
        self.commands.push(command);
        self
    }

    /// The Command Definitions, ready to send with `BotClient::bulk_overwrite_commands`
    pub fn build(self) -> Vec<ApplicationCommand> {
        self.commands
    }
}

/// Builds one chat-input (slash) Command Definition
pub struct CommandBuilder {
    name: String,
    description: String,
    default_member_permissions: Option<Permissions>,
    dm_permission: Option<bool>,
    integration_types: Option<Vec<ApplicationIntegrationType>>,
    contexts: Option<Vec<InteractionContextType>>,
    options: Option<Vec<ApplicationCommandOption>>,
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandBuilder {
    /// An unnamed Command without Options
    pub fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            options: None,
            default_member_permissions: None,
            dm_permission: None,
            integration_types: None,
            contexts: None,
        }
    }

    /// The name: 1-32 lowercase characters
    pub fn name(mut self, name: &str) -> Self {
        self.name.clear();
        self.name.push_str(name);
        self
    }

    /// The description: 1-100 characters
    pub fn description(mut self, description: &str) -> Self {
        self.description.clear();
        self.description.push_str(description);
        self
    }

    /// Adds an Option (max 25)
    pub fn add_option(mut self, option: ApplicationCommandOption) -> Self {
        match self.options {
            None => self.options = Some(vec![option]),
            Some(ref mut options) => options.push(option),
        }
        self
    }

    /// Adds a subcommand, built by `subcommand_builder`
    pub fn add_subcommand<F>(self, subcommand_builder: F) -> Self
    where
        F: FnOnce(SubcommandBuilder) -> SubcommandBuilder,
    {
        let option = subcommand_builder(SubcommandBuilder::new());
        self.add_option(option.build())
    }

    /// Adds a subcommand group, built by `subcommand_group_builder`
    pub fn add_subcommand_group<F>(self, subcommand_group_builder: F) -> Self
    where
        F: FnOnce(SubcommandGroupBuilder) -> SubcommandGroupBuilder,
    {
        let option = subcommand_group_builder(SubcommandGroupBuilder::new());
        self.add_option(option.build())
    }

    /// Permissions a member needs to see the Command, unless a server admin changes it
    pub fn with_default_member_permissions(mut self, permissions: Permissions) -> Self {
        self.default_member_permissions = Some(permissions);
        self
    }

    /// Whether the Command works in DMs; replaced by contexts
    #[deprecated(note = "Discord deprecated dm_permission; use with_contexts")]
    pub fn with_dm_permission(mut self, dm_permission: bool) -> Self {
        self.dm_permission = Some(dm_permission);
        self
    }

    /// Installation contexts (guild and/or user install) where the command is available.
    /// Global commands only.
    pub fn with_integration_types(
        mut self,
        integration_types: &[ApplicationIntegrationType],
    ) -> Self {
        self.integration_types = Some(integration_types.to_vec());
        self
    }

    /// Interaction contexts (guild, bot DM, private channel) where the command can be used.
    /// Global commands only; guild commands don't support `BotDm`.
    pub fn with_contexts(mut self, contexts: &[InteractionContextType]) -> Self {
        self.contexts = Some(contexts.to_vec());
        self
    }

    /// The Command Definition
    pub fn build_chat_command(self) -> ApplicationCommand {
        ApplicationCommand::ChatInputCommand(ChatInputCommand {
            details: CommandDetails {
                t: TypeField,
                id: None,
                application_id: None,
                guild_id: None,
                name: self.name,
                name_localizations: None,
                default_member_permissions: self.default_member_permissions,
                dm_permission: self.dm_permission,
                integration_types: self.integration_types,
                contexts: self.contexts,
                nsfw: None,
                version: None,
            },
            description: self.description,
            description_localizations: None,
            options: self.options,
        })
    }
}

/// Builds a subcommand of a [`CommandBuilder`] or [`SubcommandGroupBuilder`]
pub struct SubcommandBuilder {
    name: String,
    description: String,
    options: Option<Vec<SubcommandCommandOption>>,
}

impl Default for SubcommandBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubcommandBuilder {
    /// An unnamed subcommand without Options
    pub fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            options: None,
        }
    }

    /// The name: 1-32 lowercase characters
    pub fn name(mut self, name: &str) -> Self {
        self.name.clear();
        self.name.push_str(name);
        self
    }

    /// The description: 1-100 characters
    pub fn description(mut self, description: &str) -> Self {
        self.description.clear();
        self.description.push_str(description);
        self
    }

    /// Adds an Option (max 25)
    pub fn add_option(mut self, option: SubcommandCommandOption) -> Self {
        match self.options {
            None => self.options = Some(vec![option]),
            Some(ref mut options) => options.push(option),
        }
        self
    }

    fn build(self) -> ApplicationCommandOption {
        ApplicationCommandOption::new_subcommand_option(self.name, self.description, self.options)
    }

    fn build_subcommand(self) -> SubcommandOption {
        SubcommandOption {
            name: self.name,
            description: self.description,
            options: self.options,
            t: TypeField,
            description_localizations: None,
            name_localizations: None,
        }
    }
}

/// Builds a subcommand group of a [`CommandBuilder`]
pub struct SubcommandGroupBuilder {
    name: String,
    description: String,
    subcommands: Option<Vec<SubcommandOption>>,
}

impl Default for SubcommandGroupBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SubcommandGroupBuilder {
    /// An unnamed group without subcommands
    pub fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            subcommands: None,
        }
    }

    /// The name: 1-32 lowercase characters
    pub fn name(mut self, name: &str) -> Self {
        self.name.clear();
        self.name.push_str(name);
        self
    }

    /// The description: 1-100 characters
    pub fn description(mut self, description: &str) -> Self {
        self.description.clear();
        self.description.push_str(description);
        self
    }

    /// Adds a subcommand, built by `subcommand_builder` (max 25)
    pub fn add_subcommand<F>(mut self, subcommand_builder: F) -> Self
    where
        F: FnOnce(SubcommandBuilder) -> SubcommandBuilder,
    {
        let option = subcommand_builder(SubcommandBuilder::new()).build_subcommand();
        match self.subcommands {
            None => self.subcommands = Some(vec![option]),
            Some(ref mut options) => options.push(option),
        }
        self
    }

    fn build(self) -> ApplicationCommandOption {
        ApplicationCommandOption::new_subcommand_group_option(
            self.name,
            self.description,
            self.subcommands,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::ApplicationCommandOption;

    #[test]
    pub fn build_commands_test() {
        // arrange
        let builder = CommandsBuilder::new(Snowflake::default(), None).add_command(|builder| {
            builder.name("name").description("description").add_option(
                ApplicationCommandOption::new_string_option(
                    "name".into(),
                    "description".into(),
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
            )
        });
        // act
        let commands = builder.build();

        // assert
        assert_eq!(1, commands.len());
        let cmd = commands[0].as_chat_input_command().unwrap();
        assert_eq!("name", cmd.details.name);
        assert_eq!("description", cmd.description);
    }

    #[test]
    pub fn build_subcommands_test() {
        // arrange
        let builder = CommandsBuilder::new(Snowflake::default(), None).add_command(|builder| {
            builder
                .name("name")
                .description("description")
                .add_subcommand(|subcommand| subcommand.name("sub").description("description"))
        });

        // act
        let commands = builder.build();

        // assert
        assert_eq!(1, commands.len());
        let cmd = commands[0].as_chat_input_command().unwrap();
        assert_eq!("name", cmd.details.name);
        assert_eq!("description", cmd.description);
        assert!(matches!(
            cmd.options.as_ref().unwrap()[0],
            ApplicationCommandOption::Subcommand(_)
        ));
    }

    #[test]
    pub fn build_subcommand_group_test() {
        // arrange
        let builder = CommandsBuilder::new(Snowflake::default(), None).add_command(|builder| {
            builder
                .name("name")
                .description("description")
                .add_subcommand_group(|group| {
                    group
                        .name("group")
                        .description("group description")
                        .add_subcommand(|sub| sub.name("sub").description("sub description"))
                })
        });

        // act
        let commands = builder.build();

        // assert
        assert_eq!(1, commands.len());
        let cmd = commands[0].as_chat_input_command().unwrap();
        assert_eq!("name", cmd.details.name);
        assert_eq!("description", cmd.description);
        assert!(matches!(
            cmd.options.as_ref().unwrap()[0],
            ApplicationCommandOption::SubcommandGroup(_)
        ));
    }
}
