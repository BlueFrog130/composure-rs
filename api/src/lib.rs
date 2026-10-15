use composure_commands::command::{ApplicationCommand, CommandsBuilder};
use std::{thread, time::Duration};

use reqwest::{
    IntoUrl, StatusCode,
    blocking::RequestBuilder,
    header::{self, AUTHORIZATION},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

mod application_commands;

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
                .map_err(Error::HeaderError)?,
        );

        let client = reqwest::blocking::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(Error::RequestError)?;

        Ok(DiscordClient {
            client,
            application_id: application_id.to_string(),
        })
    }

    fn get<T, U: DeserializeOwned>(&self, url: T) -> Result<U>
    where
        T: IntoUrl,
    {
        let url = url.into_url().map_err(Error::RequestError)?;
        self.send(|| self.client.get(url.clone()))
    }

    fn post<T, U, R: DeserializeOwned>(&self, url: T, body: &U) -> Result<R>
    where
        T: IntoUrl,
        U: Serialize,
    {
        let url = url.into_url().map_err(Error::RequestError)?;
        self.send(|| self.client.post(url.clone()).json(body))
    }

    fn put<T, U, R: DeserializeOwned>(&self, url: T, body: &U) -> Result<R>
    where
        T: IntoUrl,
        U: Serialize,
    {
        let url = url.into_url().map_err(Error::RequestError)?;
        self.send(|| self.client.put(url.clone()).json(body))
    }

    /// Sends the request, waiting out and retrying [rate limits](https://discord.com/developers/docs/topics/rate-limits)
    /// up to [`MAX_RATE_LIMIT_RETRIES`] times. Waits longer than [`MAX_RATE_LIMIT_WAIT`] are returned as errors.
    fn send<R: DeserializeOwned>(&self, request: impl Fn() -> RequestBuilder) -> Result<R> {
        for _ in 0..MAX_RATE_LIMIT_RETRIES {
            let response = request().send().map_err(Error::RequestError)?;
            if response.status() != StatusCode::TOO_MANY_REQUESTS {
                return handle_response(response);
            }

            let body = response.text().map_err(Error::RequestError)?;
            match serde_json::from_str::<RateLimited>(&body) {
                Ok(limited) if limited.retry_after <= MAX_RATE_LIMIT_WAIT.as_secs_f64() => {
                    thread::sleep(Duration::from_secs_f64(limited.retry_after));
                }
                _ => return Err(Error::UnknownResponse(body)),
            }
        }

        handle_response(request().send().map_err(Error::RequestError)?)
    }
}

const MAX_RATE_LIMIT_RETRIES: u32 = 3;
const MAX_RATE_LIMIT_WAIT: Duration = Duration::from_secs(60);

/// Body of a 429 response
#[derive(Deserialize)]
struct RateLimited {
    /// Seconds to wait before retrying
    retry_after: f64,
}

/// 401 is [`Error::Unauthorized`], other non-2xx statuses are [`Error::UnknownResponse`] with the body.
fn handle_response<R: DeserializeOwned>(response: reqwest::blocking::Response) -> Result<R> {
    match response.status() {
        StatusCode::UNAUTHORIZED => Err(Error::Unauthorized),
        status if status.is_success() => response.json().map_err(Error::RequestError),
        _ => Err(Error::UnknownResponse(
            response.text().map_err(Error::RequestError)?,
        )),
    }
}

pub trait UpdateCommands {
    fn update_commands(&self, token: &str) -> Result<Vec<ApplicationCommand>>;
}

impl UpdateCommands for CommandsBuilder {
    fn update_commands(&self, token: &str) -> Result<Vec<ApplicationCommand>> {
        let client = DiscordClient::new(token, &self.application_id.to_string())?;

        let ref_vec = self.commands.iter().collect();

        let updated_commands = match &self.guild_id {
            Some(snowflake) => client.overwrite_guild_commands(&snowflake.to_string(), &ref_vec),
            None => client.overwrite_global_commands(&ref_vec),
        }?;

        Ok(updated_commands)
    }
}
