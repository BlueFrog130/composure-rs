use composure::models::{Permissions, Snowflake, TypeField};

use crate::command::*;

pub struct CommandsBuilder {
    pub application_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub commands: Vec<ApplicationCommand>,
}

impl CommandsBuilder {
    pub fn new(application_id: Snowflake, guild_id: Option<Snowflake>) -> Self {
        Self {
            commands: Vec::new(),
            application_id,
            guild_id,
        }
    }

    pub fn add_command<F>(mut self, command_builder: F) -> Self
    where
        F: FnOnce(CommandBuilder) -> CommandBuilder,
    {
        let command = command_builder(CommandBuilder::new()).build_chat_command();
        self.commands.push(command);
        self
    }

    pub fn build(self) -> Vec<ApplicationCommand> {
        self.commands
    }
}

pub struct CommandBuilder {
    name: String,
    description: String,
    default_member_permissions: Option<Permissions>,
    dm_permission: Option<bool>,
    options: Option<Vec<ApplicationCommandOption>>,
}

impl CommandBuilder {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            options: None,
            default_member_permissions: None,
            dm_permission: None,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name.clear();
        self.name.push_str(name);
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description.clear();
        self.description.push_str(description);
        self
    }

    pub fn add_option(mut self, option: ApplicationCommandOption) -> Self {
        match self.options {
            None => self.options = Some(vec![option]),
            Some(ref mut options) => options.push(option),
        }
        self
    }

    pub fn add_subcommand<F>(self, subcommand_builder: F) -> Self
    where
        F: FnOnce(SubcommandBuilder) -> SubcommandBuilder,
    {
        let option = subcommand_builder(SubcommandBuilder::new());
        self.add_option(option.build())
    }

    pub fn add_subcommand_group<F>(self, subcommand_group_builder: F) -> Self
    where
        F: FnOnce(SubcommandGroupBuilder) -> SubcommandGroupBuilder,
    {
        let option = subcommand_group_builder(SubcommandGroupBuilder::new());
        self.add_option(option.build())
    }

    pub fn with_default_member_permissions(mut self, permissions: Permissions) -> Self {
        self.default_member_permissions = Some(permissions);
        self
    }

    pub fn with_dm_permission(mut self, dm_permission: bool) -> Self {
        self.dm_permission = Some(dm_permission);
        self
    }

    pub fn build_chat_command(self) -> ApplicationCommand {
        ApplicationCommand::new_chat_input_command(
            self.name,
            self.description,
            self.default_member_permissions,
            self.dm_permission,
            None,
            self.options,
        )
    }
}

pub struct SubcommandBuilder {
    name: String,
    description: String,
    options: Option<Vec<SubcommandCommandOption>>,
}

impl SubcommandBuilder {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            options: None,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name.clear();
        self.name.push_str(name);
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description.clear();
        self.description.push_str(description);
        self
    }

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

pub struct SubcommandGroupBuilder {
    name: String,
    description: String,
    subcommands: Option<Vec<SubcommandOption>>,
}

impl SubcommandGroupBuilder {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            subcommands: None,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name.clear();
        self.name.push_str(name);
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description.clear();
        self.description.push_str(description);
        self
    }

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
