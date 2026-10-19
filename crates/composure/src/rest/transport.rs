use std::{borrow::Cow, fmt, rc::Rc, time::Duration};

/// The Adapter-provided way to send one HTTP request and to sleep; the REST client's only contact
/// with the platform (`worker::Fetch` on Cloudflare, `reqwest` natively).
///
/// Implementations send exactly the given request: no retries, no redirects to follow for Discord,
/// no status handling. Non-2xx statuses are a normal [`RawResponse`], only failures to get any
/// response at all are a [`TransportError`].
#[allow(async_fn_in_trait)] // dispatch is static and Workers futures are not Send
pub trait Transport {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError>;

    /// Waits out a rate limit
    async fn sleep(&self, duration: Duration);
}

impl<T: Transport + ?Sized> Transport for &T {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        (**self).send(request).await
    }

    async fn sleep(&self, duration: Duration) {
        (**self).sleep(duration).await
    }
}

impl<T: Transport + ?Sized> Transport for Rc<T> {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        (**self).send(request).await
    }

    async fn sleep(&self, duration: Duration) {
        (**self).sleep(duration).await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Patch => "PATCH",
            Method::Delete => "DELETE",
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One fully-built HTTP request: absolute URL, every header (including `Content-Type` when there is
/// a body) and the encoded body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub method: Method,
    pub url: String,
    /// Values are borrowed when constant (`User-Agent`, `Content-Type: application/json`)
    pub headers: Vec<(&'static str, Cow<'static, str>)>,
    pub body: Option<Vec<u8>>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_ref())
    }
}

/// The response headers the client reads (rate limits) plus `content-type`. A Transport on a
/// platform where reading headers is costly (each one crosses into JS on Workers) may keep only
/// these in [`RawResponse::headers`].
pub const DISCORD_HEADERS: &[&str] = &[
    "content-type",
    "retry-after",
    "x-ratelimit-bucket",
    "x-ratelimit-global",
    "x-ratelimit-limit",
    "x-ratelimit-remaining",
    "x-ratelimit-reset",
    "x-ratelimit-reset-after",
    "x-ratelimit-scope",
];

/// An HTTP response as received, any status
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawResponse {
    pub status: u16,
    /// Header names as sent; looked up case-insensitively by [`RawResponse::header`]
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl RawResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }
}

/// No response was received (network failure, platform error, ...)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportError(pub String);

impl TransportError {
    pub fn new(error: impl fmt::Display) -> Self {
        Self(error.to_string())
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for TransportError {}
