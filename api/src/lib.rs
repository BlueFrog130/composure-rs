use std::collections::HashMap;

use composure::models::Snowflake;
use composure_commands::command::ApplicationCommand;
use reqwest::{
    header::{self, AUTHORIZATION},
    IntoUrl, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};

mod application_commands;

pub use application_commands::*;

pub const DISCORD_API: &str = "https://discord.com/api/v10";

#[derive(Debug)]
pub enum Error {
    RequestError(reqwest::Error),
    HeaderError(header::InvalidHeaderValue),
    Unauthorized,
    UnknownResponse(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct DiscordClient {
    client: reqwest::blocking::Client,
    application_id: String,
}

impl DiscordClient {
    pub fn new(token: &str, application_id: &str) -> Result<DiscordClient> {
        let mut headers = header::HeaderMap::new();

        headers.insert(
            AUTHORIZATION,
            header::HeaderValue::from_str(format!("Bot {token}").as_str())
                .map_err(|e| Error::HeaderError(e))?,
        );

        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| Error::RequestError(e))?;

        Ok(DiscordClient {
            client,
            application_id: application_id.to_string(),
        })
    }

    fn get<T, U: DeserializeOwned>(&self, url: T) -> Result<U>
    where
        T: IntoUrl,
    {
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|e| Error::RequestError(e))?;

        match response.status() {
            StatusCode::UNAUTHORIZED => Err(Error::Unauthorized),
            _ => Ok(response.json().map_err(|e| Error::RequestError(e))?),
        }
    }

    fn post<T, U, R: DeserializeOwned>(&self, url: T, body: &U) -> Result<R>
    where
        T: IntoUrl,
        U: Serialize,
    {
        let response = self
            .client
            .post(url)
            .json(body)
            .send()
            .map_err(|e| Error::RequestError(e))?;

        match response.status() {
            StatusCode::UNAUTHORIZED => Err(Error::Unauthorized),
            _ => Ok(response.json().map_err(|e| Error::RequestError(e))?),
        }
    }

    fn put<T, U, R: DeserializeOwned>(&self, url: T, body: &U) -> Result<R>
    where
        T: IntoUrl,
        U: Serialize,
    {
        let response = self
            .client
            .put(url)
            .json(body)
            .send()
            .map_err(|e| Error::RequestError(e))?;

        match response.status() {
            StatusCode::UNAUTHORIZED => Err(Error::Unauthorized),
            StatusCode::OK | StatusCode::CREATED => {
                Ok(response.json().map_err(|e| Error::RequestError(e))?)
            }
            _ => Err(Error::UnknownResponse(
                response.text().map_err(|e| Error::RequestError(e))?,
            )),
        }
    }
}

pub fn update_commands(
    token: &str,
    application_id: &str,
    commands: &[ApplicationCommand],
) -> Result<Vec<ApplicationCommand>> {
    let client = DiscordClient::new(token, application_id)?;

    let mut groups: HashMap<&Option<Snowflake>, Vec<&ApplicationCommand>> = HashMap::new();

    for command in commands.iter() {
        let group = groups.get_mut(command.get_guild_id());

        match group {
            None => {
                groups.insert(command.get_guild_id(), vec![command]);
            }
            Some(group) => {
                group.push(command);
            }
        }
    }

    let mut updated_commands: Vec<ApplicationCommand> = vec![];

    for (guild_id, group) in groups.iter() {
        let updated_group = match guild_id {
            Some(snowflake) => client.overwrite_guild_commands(&snowflake.to_string(), group),
            None => client.overwrite_global_commands(group),
        }?;

        updated_commands.extend(updated_group);
    }

    Ok(updated_commands)
}

#[cfg(test)]
pub mod tests {
    use std::{env, str::FromStr};

    use composure::models::Snowflake;

    use super::*;

    fn setup() {
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
    pub fn update_commands_test() {
        setup();

        let guild_id = Snowflake::from_str(&guild_id()).unwrap();
        let application_id = Snowflake::from_str(&application_id()).unwrap();

        let commands = [
            ApplicationCommand::new_chat_input_command(
                application_id.clone(),
                None,
                String::from("test"),
                String::from("descr"),
                None,
                None,
                None,
                None,
            ),
            ApplicationCommand::new_message_command(
                application_id.clone(),
                Some(guild_id.clone()),
                String::from("msg command"),
                None,
                None,
                None,
            ),
            ApplicationCommand::new_chat_input_command(
                application_id.clone(),
                Some(guild_id),
                String::from("guild_command"),
                String::from("guild command desc"),
                None,
                None,
                None,
                None,
            ),
            ApplicationCommand::new_chat_input_command(
                application_id.clone(),
                None,
                String::from("test_2"),
                String::from("descr"),
                None,
                None,
                None,
                None,
            ),
        ];

        let updated_commands =
            update_commands(&token(), &application_id.to_string(), &commands).unwrap();

        println!("{:#?}", updated_commands);
    }
}
