use std::{sync::Arc, time::Duration};

use composure::rest::{self, DISCORD_API, DISCORD_HEADERS, RawResponse, Transport, TransportError};

/// [`Transport`] over an async [`reqwest::Client`] (rustls), sleeping on tokio's timer. Responses
/// keep only the [`DISCORD_HEADERS`] (rate limits and `content-type`). Cloning shares the client's
/// connection pool.
#[derive(Debug, Clone, Default)]
pub struct HttpTransport {
    client: reqwest::Client,
    /// Replaces [`DISCORD_API`] in request URLs
    api_base: Option<Arc<str>>,
}

impl HttpTransport {
    /// A Transport with a default `reqwest` client
    pub fn new() -> Self {
        Self::default()
    }

    /// Sends through `client`, e.g. one the rest of the server already uses
    pub fn with_client(client: reqwest::Client) -> Self {
        Self {
            client,
            api_base: None,
        }
    }

    /// Sends what would go to Discord's API (`https://discord.com/api/v10`) to `base` instead:
    /// a proxy, or a stand-in Discord in tests. Applies to every client built on this Transport,
    /// Interaction, Bot and Background Work alike.
    pub fn with_api_base(mut self, base: impl Into<String>) -> Self {
        let base = base.into();
        self.api_base = Some(base.trim_end_matches('/').into());
        self
    }

    /// The `reqwest` client requests go through
    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

impl Transport for HttpTransport {
    async fn send(&self, request: &rest::Request) -> Result<RawResponse, TransportError> {
        let method = match request.method {
            rest::Method::Get => reqwest::Method::GET,
            rest::Method::Post => reqwest::Method::POST,
            rest::Method::Put => reqwest::Method::PUT,
            rest::Method::Patch => reqwest::Method::PATCH,
            rest::Method::Delete => reqwest::Method::DELETE,
        };
        let url = match (&self.api_base, request.url.strip_prefix(DISCORD_API)) {
            (Some(base), Some(path)) => format!("{base}{path}"),
            _ => request.url.clone(),
        };
        let mut builder = self.client.request(method, url);
        for (name, value) in &request.headers {
            builder = builder.header(*name, value.as_ref());
        }
        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }
        let response = builder.send().await.map_err(TransportError::new)?;
        let status = response.status().as_u16();
        let received = response.headers();
        let headers = DISCORD_HEADERS
            .iter()
            .filter_map(|&name| {
                let value = received.get(name)?.to_str().ok()?;
                Some((name.to_string(), value.to_string()))
            })
            .collect();
        let body = response
            .bytes()
            .await
            .map_err(TransportError::new)?
            .to_vec();
        Ok(RawResponse {
            status,
            headers,
            body,
        })
    }

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await
    }
}
