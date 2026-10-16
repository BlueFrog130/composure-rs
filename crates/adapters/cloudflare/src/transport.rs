use std::time::Duration;

use composure::rest::{self, RawResponse, Transport, TransportError};
use worker::{Delay, Fetch, Headers, Method, Request, RequestInit, js_sys::Uint8Array};

/// [`Transport`] over Workers' `fetch`, sleeping with a JS timer
#[derive(Debug, Clone, Copy, Default)]
pub struct FetchTransport;

impl Transport for FetchTransport {
    async fn send(&self, request: &rest::Request) -> Result<RawResponse, TransportError> {
        let headers = Headers::new();
        for (name, value) in &request.headers {
            headers.set(name, value).map_err(TransportError::new)?;
        }
        let mut init = RequestInit::new();
        init.with_method(match request.method {
            rest::Method::Get => Method::Get,
            rest::Method::Post => Method::Post,
            rest::Method::Put => Method::Put,
            rest::Method::Patch => Method::Patch,
            rest::Method::Delete => Method::Delete,
        })
        .with_headers(headers)
        .with_body(
            request
                .body
                .as_deref()
                .map(|body| Uint8Array::from(body).into()),
        );
        let request = Request::new_with_init(&request.url, &init).map_err(TransportError::new)?;
        let mut response = Fetch::Request(request)
            .send()
            .await
            .map_err(TransportError::new)?;
        Ok(RawResponse {
            status: response.status_code(),
            headers: response.headers().entries().collect(),
            body: response.bytes().await.map_err(TransportError::new)?,
        })
    }

    async fn sleep(&self, duration: Duration) {
        Delay::from(duration).await
    }
}
