use composure_commands::command::{ApplicationCommand, CommandsBuilder};
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

pub trait UpdateCommands {
    fn update_commands(&self, token: &str) -> Result<Vec<ApplicationCommand>>;
}

impl UpdateCommands for CommandsBuilder {
    fn update_commands(&self, token: &str) -> Result<Vec<ApplicationCommand>> {
        let client = DiscordClient::new(token, &self.application_id.to_string())?;

        let ref_vec = self.commands.iter().map(|c| c).collect();

        let updated_commands = match &self.guild_id {
            Some(snowflake) => client.overwrite_guild_commands(&snowflake.to_string(), &ref_vec),
            None => client.overwrite_global_commands(&ref_vec),
        }?;

        Ok(updated_commands)
    }
}
