use commands::command::ApplicationCommand;

use crate::{DiscordClient, Error, Result, DISCORD_API};

impl DiscordClient {
    pub fn get_global_commands(&self) -> Result<Vec<ApplicationCommand>> {
        let url = format!(
            "{DISCORD_API}/applications/{}/commands",
            self.application_id
        );
        let commands: Vec<ApplicationCommand> = self.get(url)?;
        Ok(commands)
    }

    pub fn get_guild_commands(&self, guild_id: &str) -> Result<Vec<ApplicationCommand>> {
        let url = format!(
            "{DISCORD_API}/applications/{}/guilds/{}/commands",
            self.application_id, guild_id
        );
        let commands: Vec<ApplicationCommand> = self.get(url)?;
        Ok(commands)
    }

    pub fn create_global_command(
        &self,
        command: &ApplicationCommand,
    ) -> Result<ApplicationCommand> {
        let url = format!(
            "{DISCORD_API}/applications/{}/commands",
            self.application_id
        );

        let command = self.post(url, command)?;

        Ok(command)
    }

    pub fn create_guild_command(
        &self,
        guild_id: &str,
        command: &ApplicationCommand,
    ) -> Result<ApplicationCommand> {
        let url = format!(
            "{DISCORD_API}/applications/{}/guilds/{}/commands",
            self.application_id, guild_id
        );

        let command = self.post(url, command)?;

        Ok(command)
    }

    /// Sets the list of global commands.
    ///
    /// WARNING: All existing commands will be deleted
    pub fn overwrite_global_commands(
        &self,
        commands: &Vec<&ApplicationCommand>,
    ) -> Result<Vec<ApplicationCommand>> {
        let url = format!(
            "{DISCORD_API}/applications/{}/commands",
            self.application_id
        );

        let response = self.put(url, commands);

        if let Err(ref e) = response {
            if let Error::UnknownResponse(response) = e {
                println!("Invalid response: {}", response);
            }
        }

        response
    }

    /// Sets the list of guild commands.
    ///
    /// WARNING: All existing commands will be deleted
    pub fn overwrite_guild_commands(
        &self,
        guild_id: &str,
        commands: &Vec<&ApplicationCommand>,
    ) -> Result<Vec<ApplicationCommand>> {
        let url = format!(
            "{DISCORD_API}/applications/{}/guilds/{}/commands",
            self.application_id, guild_id
        );

        let commands = self.put(url, commands)?;

        Ok(commands)
    }
}

#[cfg(test)]
pub mod tests {
    use interaction_bot::models::Snowflake;
    use std::{env, str::FromStr};

    use super::*;

    fn setup<'a>() {
        dotenv::from_filename(".env.test").unwrap();
    }

    fn application_id() -> String {
        env::var("DISCORD_APPLICATION_ID").unwrap()
    }

    fn guild_id() -> String {
        env::var("DISCORD_GUILD_ID").unwrap()
    }

    fn token() -> String {
        env::var("DISCORD_TOKEN").unwrap()
    }

    #[test]
    pub fn global_commands() {
        setup();
        let client = DiscordClient::new(&token(), &application_id()).unwrap();
        let commands = client.get_global_commands();
        println!("{:#?}", commands);
    }

    #[test]
    pub fn guild_commands() {
        setup();
        let client = DiscordClient::new(&token(), &application_id()).unwrap();
        let commands = client.get_guild_commands(&guild_id());
        println!("{:#?}", commands);
    }

    #[test]
    pub fn create_global_command() {
        setup();

        let application_id = application_id();

        let client = DiscordClient::new(&token(), &application_id).unwrap();

        let command = ApplicationCommand::new_chat_input_command(
            Snowflake::from_str(&application_id).unwrap(),
            None,
            String::from("test"),
            String::from("test"),
            None,
            None,
            None,
            None,
        );

        let command = client.create_global_command(&command).unwrap();

        println!("{:#?}", command);
    }

    #[test]
    pub fn create_guild_command() {
        setup();

        let application_id = application_id();

        let client = DiscordClient::new(&token(), &application_id).unwrap();

        let command = ApplicationCommand::new_user_command(
            Snowflake::from_str(&application_id).unwrap(),
            None,
            String::from("test"),
            None,
            None,
            None,
        );

        println!("{}", serde_json::to_string_pretty(&command).unwrap());

        let command = client.create_guild_command(&guild_id(), &command).unwrap();

        println!("{:#?}", command);
    }

    #[test]
    pub fn overwrite_global_command() {
        setup();

        let application_id = application_id();

        let client = DiscordClient::new(&token(), &application_id).unwrap();

        let binding = ApplicationCommand::new_chat_input_command(
            Snowflake::from_str(&application_id).unwrap(),
            None,
            String::from("test"),
            String::from("test"),
            None,
            None,
            None,
            None,
        );
        let commands = vec![&binding];

        let command = client.overwrite_global_commands(&commands).unwrap();

        println!("{:#?}", command);
    }

    #[test]
    pub fn overwrite_guild_command() {
        setup();

        let application_id = application_id();

        let client = DiscordClient::new(&token(), &application_id).unwrap();

        let binding = ApplicationCommand::new_chat_input_command(
            Snowflake::from_str(&application_id).unwrap(),
            None,
            String::from("test"),
            String::from("test"),
            None,
            None,
            None,
            None,
        );
        let commands = vec![&binding];

        let command = client
            .overwrite_guild_commands(&guild_id(), &commands)
            .unwrap();

        println!("{:#?}", command);
    }
}
