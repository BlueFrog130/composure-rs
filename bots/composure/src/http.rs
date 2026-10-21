//! JSON GETs to APIs other than Discord's (GitHub, crates.io), over the Platform's
//! [`Transport`], so tests answer them like Discord's.

use std::borrow::Cow;

use composure::{
    command::HandlerError,
    rest::{Method, Request, Transport},
};
use serde::de::DeserializeOwned;

/// Sent with every request; GitHub and crates.io both require one
pub const USER_AGENT: &str = "composure-bot (https://github.com/BlueFrog130/composure-rs)";

/// GETs `url` and decodes the JSON body; a non-2xx status is an error naming it
pub async fn get_json<T: DeserializeOwned>(
    transport: &impl Transport,
    url: String,
    headers: Vec<(&'static str, Cow<'static, str>)>,
) -> Result<T, HandlerError> {
    let mut request = Request {
        method: Method::Get,
        url,
        headers,
        body: None,
    };
    request
        .headers
        .push(("user-agent", Cow::Borrowed(USER_AGENT)));
    request
        .headers
        .push(("accept", Cow::Borrowed("application/json")));
    let response = transport
        .send(&request)
        .await
        .map_err(|error| HandlerError::new(format_args!("GET {}: {error}", request.url)))?;
    if !(200..300).contains(&response.status) {
        return Err(HandlerError::new(format_args!(
            "GET {}: status {}",
            request.url, response.status
        )));
    }
    serde_json::from_slice(&response.body)
        .map_err(|error| HandlerError::new(format_args!("GET {}: {error}", request.url)))
}
