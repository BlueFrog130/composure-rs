use composure_commands::command::ApplicationCommand;

use crate::{DISCORD_API, DiscordClient, Result};

impl DiscordClient {
    pub fn get_global_commands(&self) -> Result<Vec<ApplicationCommand>> {
        let url = format!(
            "{DISCORD_API}/applications/{}/commands?with_localizations=true",
            self.application_id
        );
        let commands: Vec<ApplicationCommand> = self.get(url)?;
        Ok(commands)
    }

    pub fn get_guild_commands(&self, guild_id: &str) -> Result<Vec<ApplicationCommand>> {
        let url = format!(
            "{DISCORD_API}/applications/{}/guilds/{}/commands?with_localizations=true",
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

        let commands = self.put(url, commands)?;

        Ok(commands)
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
