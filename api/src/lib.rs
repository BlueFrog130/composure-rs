use commands::command::ApplicationCommand;
use models::common::Snowflake;

const DISCORD_API: &str = "https://discord.com/api/v10";

type Result<T> = std::result::Result<T, reqwest::Error>;

pub fn get_global_commands(
    application_id: &str,
    bot_token: &str,
) -> Result<Vec<ApplicationCommand>> {
    let url = format!("{DISCORD_API}/applications/{application_id}/commands");
    let commands: Vec<ApplicationCommand> = reqwest::blocking::get(url)?.json()?;
    todo!();
    // Ok(commands)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    pub fn global_commands() {
        let commands = get_global_commands("1052322265397739523").unwrap();
        println!("{:#?}", commands);
    }
}
