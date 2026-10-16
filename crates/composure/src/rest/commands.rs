//! [Application Commands](https://discord.com/developers/docs/interactions/application-commands#endpoints)

use crate::{
    definition::ApplicationCommand,
    types::{GuildCommandPermissions, Snowflake},
};

use super::{Body, BotClient, Response, Result, Route, Transport, route::Query, route::routes};

routes! {
    GET_GLOBAL_COMMANDS = Get "/applications/{application_id}/commands";
    CREATE_GLOBAL_COMMAND = Post "/applications/{application_id}/commands";
    OVERWRITE_GLOBAL_COMMANDS = Put "/applications/{application_id}/commands";
    GET_GLOBAL_COMMAND = Get "/applications/{application_id}/commands/{command_id}";
    EDIT_GLOBAL_COMMAND = Patch "/applications/{application_id}/commands/{command_id}";
    DELETE_GLOBAL_COMMAND = Delete "/applications/{application_id}/commands/{command_id}";
    GET_GUILD_COMMANDS = Get "/applications/{application_id}/guilds/{guild_id}/commands";
    CREATE_GUILD_COMMAND = Post "/applications/{application_id}/guilds/{guild_id}/commands";
    OVERWRITE_GUILD_COMMANDS = Put "/applications/{application_id}/guilds/{guild_id}/commands";
    GET_GUILD_COMMAND = Get "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}";
    EDIT_GUILD_COMMAND = Patch "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}";
    DELETE_GUILD_COMMAND = Delete "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}";
    GET_GUILD_COMMAND_PERMISSIONS = Get "/applications/{application_id}/guilds/{guild_id}/commands/permissions";
    GET_COMMAND_PERMISSIONS = Get "/applications/{application_id}/guilds/{guild_id}/commands/{command_id}/permissions";
}

/// Where a Command Definition lives on Discord
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandScope {
    Global,
    Guild(Snowflake),
}

impl<T: Transport> BotClient<T> {
    fn commands_url(&self, global: Route, guild: Route, scope: CommandScope) -> (Route, String) {
        match scope {
            CommandScope::Global => (global, self.rest.url(global, &[&self.application_id])),
            CommandScope::Guild(guild_id) => (
                guild,
                self.rest.url(guild, &[&self.application_id, &guild_id]),
            ),
        }
    }

    fn command_url(
        &self,
        global: Route,
        guild: Route,
        scope: CommandScope,
        command_id: Snowflake,
    ) -> (Route, String) {
        match scope {
            CommandScope::Global => (
                global,
                self.rest.url(global, &[&self.application_id, &command_id]),
            ),
            CommandScope::Guild(guild_id) => (
                guild,
                self.rest
                    .url(guild, &[&self.application_id, &guild_id, &command_id]),
            ),
        }
    }

    /// [Get Global](https://discord.com/developers/docs/interactions/application-commands#get-global-application-commands) /
    /// [Guild Application Commands](https://discord.com/developers/docs/interactions/application-commands#get-guild-application-commands),
    /// with localizations
    pub async fn get_commands(
        &self,
        scope: CommandScope,
    ) -> Result<Response<Vec<ApplicationCommand>>> {
        let (route, mut url) = self.commands_url(GET_GLOBAL_COMMANDS, GET_GUILD_COMMANDS, scope);
        Query::new(&mut url).push("with_localizations", true);
        self.rest.call(route, url, Body::NONE, None).await
    }

    /// [Create Global](https://discord.com/developers/docs/interactions/application-commands#create-global-application-command) /
    /// [Guild Application Command](https://discord.com/developers/docs/interactions/application-commands#create-guild-application-command);
    /// overwrites a Command with the same name
    pub async fn create_command(
        &self,
        scope: CommandScope,
        command: &ApplicationCommand,
    ) -> Result<Response<ApplicationCommand>> {
        let (route, url) = self.commands_url(CREATE_GLOBAL_COMMAND, CREATE_GUILD_COMMAND, scope);
        self.rest.call(route, url, Body::Json(command), None).await
    }

    /// [Bulk Overwrite Global](https://discord.com/developers/docs/interactions/application-commands#bulk-overwrite-global-application-commands) /
    /// [Guild Application Commands](https://discord.com/developers/docs/interactions/application-commands#bulk-overwrite-guild-application-commands).
    ///
    /// WARNING: Commands not in `commands` are deleted
    pub async fn overwrite_commands(
        &self,
        scope: CommandScope,
        commands: &[&ApplicationCommand],
    ) -> Result<Response<Vec<ApplicationCommand>>> {
        let (route, url) =
            self.commands_url(OVERWRITE_GLOBAL_COMMANDS, OVERWRITE_GUILD_COMMANDS, scope);
        self.rest
            .call(route, url, Body::Json(&commands), None)
            .await
    }

    /// [Get Global](https://discord.com/developers/docs/interactions/application-commands#get-global-application-command) /
    /// [Guild Application Command](https://discord.com/developers/docs/interactions/application-commands#get-guild-application-command)
    pub async fn get_command(
        &self,
        scope: CommandScope,
        command_id: Snowflake,
    ) -> Result<Response<ApplicationCommand>> {
        let (route, url) =
            self.command_url(GET_GLOBAL_COMMAND, GET_GUILD_COMMAND, scope, command_id);
        self.rest.call(route, url, Body::NONE, None).await
    }

    /// [Edit Global](https://discord.com/developers/docs/interactions/application-commands#edit-global-application-command) /
    /// [Guild Application Command](https://discord.com/developers/docs/interactions/application-commands#edit-guild-application-command)
    pub async fn edit_command(
        &self,
        scope: CommandScope,
        command_id: Snowflake,
        command: &ApplicationCommand,
    ) -> Result<Response<ApplicationCommand>> {
        let (route, url) =
            self.command_url(EDIT_GLOBAL_COMMAND, EDIT_GUILD_COMMAND, scope, command_id);
        self.rest.call(route, url, Body::Json(command), None).await
    }

    /// [Get Guild Application Command Permissions](https://discord.com/developers/docs/interactions/application-commands#get-guild-application-command-permissions)
    /// for every command in `guild_id`. Setting them needs a user's Bearer token, not the bot's.
    pub async fn get_guild_command_permissions(
        &self,
        guild_id: Snowflake,
    ) -> Result<Response<Vec<GuildCommandPermissions>>> {
        let url = self.rest.url(
            GET_GUILD_COMMAND_PERMISSIONS,
            &[&self.application_id, &guild_id],
        );
        self.rest
            .call(GET_GUILD_COMMAND_PERMISSIONS, url, Body::NONE, None)
            .await
    }

    /// [Get Application Command Permissions](https://discord.com/developers/docs/interactions/application-commands#get-application-command-permissions)
    pub async fn get_command_permissions(
        &self,
        guild_id: Snowflake,
        command_id: Snowflake,
    ) -> Result<Response<GuildCommandPermissions>> {
        let url = self.rest.url(
            GET_COMMAND_PERMISSIONS,
            &[&self.application_id, &guild_id, &command_id],
        );
        self.rest
            .call(GET_COMMAND_PERMISSIONS, url, Body::NONE, None)
            .await
    }

    /// [Delete Global](https://discord.com/developers/docs/interactions/application-commands#delete-global-application-command) /
    /// [Guild Application Command](https://discord.com/developers/docs/interactions/application-commands#delete-guild-application-command)
    pub async fn delete_command(&self, scope: CommandScope, command_id: Snowflake) -> Result<()> {
        let (route, url) = self.command_url(
            DELETE_GLOBAL_COMMAND,
            DELETE_GUILD_COMMAND,
            scope,
            command_id,
        );
        self.rest.call_empty(route, url, Body::NONE, None).await
    }
}
