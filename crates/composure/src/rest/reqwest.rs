use std::time::Duration;

use super::{Method, RawResponse, Request, Transport, TransportError};

/// Native [`Transport`] for deploy-time tools: blocking `reqwest` inside an async fn, meant to be
/// driven by a simple executor like `pollster`. Do not use inside an async runtime (tokio) or on
/// wasm.
#[derive(Debug, Clone, Default)]
pub struct ReqwestTransport {
    client: reqwest::blocking::Client,
}

impl ReqwestTransport {
    /// A Transport with a default `reqwest` client
    pub fn new() -> Self {
        Self::default()
    }

    /// A Transport over `client`
    pub fn with_client(client: reqwest::blocking::Client) -> Self {
        Self { client }
    }
}

impl Transport for ReqwestTransport {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        let method = match request.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
            Method::Put => reqwest::Method::PUT,
            Method::Patch => reqwest::Method::PATCH,
            Method::Delete => reqwest::Method::DELETE,
        };
        let mut builder = self.client.request(method, &request.url);
        for (name, value) in &request.headers {
            builder = builder.header(*name, value.as_ref());
        }
        if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        }
        let response = builder.send().map_err(TransportError::new)?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(name, value)| Some((name.to_string(), value.to_str().ok()?.to_string())))
            .collect();
        let body = response.bytes().map_err(TransportError::new)?.to_vec();
        Ok(RawResponse {
            status,
            headers,
            body,
        })
    }

    async fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}
