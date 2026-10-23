use std::{
    borrow::Cow,
    fmt::{Display, Write},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::{
    trace::{Instrument, Span},
    types::File,
};

use super::{
    ApiError, Error, Method, RateLimited, RawResponse, Request, Response, Result, Route, Transport,
    multipart,
};

/// Discord REST base URL (API v10)
pub const DISCORD_API: &str = "https://discord.com/api/v10";

const USER_AGENT: &str = concat!(
    "DiscordBot (https://github.com/BlueFrog130/composure-rs, ",
    env!("CARGO_PKG_VERSION"),
    ")"
);

/// Rate-limit retries before giving up with [`Error::RateLimited`]
const MAX_RATE_LIMIT_RETRIES: u32 = 3;

/// Default longest single wait: short, because requests run inside a platform request (and its
/// time limit); deploy-time tools raise it with `with_max_rate_limit_wait`.
pub const DEFAULT_MAX_RATE_LIMIT_WAIT: Duration = Duration::from_secs(2);

/// What every client shares: Transport, credentials, rate-limit policy
#[derive(Debug, Clone)]
pub(crate) struct Rest<T> {
    transport: T,
    authorization: Option<String>,
    base: Cow<'static, str>,
    max_rate_limit_wait: Duration,
}

/// A request body: JSON, or JSON + files as multipart
#[allow(dead_code)] // `Json` is only used by bot Endpoint features
pub(crate) enum Body<'a, B: Serialize> {
    None,
    Json(&'a B),
    WithFiles(&'a B, &'a [File]),
    /// Text fields and one file, without `payload_json`
    Form(&'a [(&'a str, &'a str)], &'a File),
}

impl Body<'_, ()> {
    pub(crate) const NONE: Self = Body::None;
}

impl<T: Transport> Rest<T> {
    pub(crate) fn new(transport: T, authorization: Option<String>) -> Self {
        Self {
            transport,
            authorization,
            base: Cow::Borrowed(DISCORD_API),
            max_rate_limit_wait: DEFAULT_MAX_RATE_LIMIT_WAIT,
        }
    }

    pub(crate) fn set_base(&mut self, base: String) {
        self.base = Cow::Owned(base);
    }

    pub(crate) fn set_max_rate_limit_wait(&mut self, wait: Duration) {
        self.max_rate_limit_wait = wait;
    }

    pub(crate) fn transport(&self) -> &T {
        &self.transport
    }

    pub(crate) fn url(&self, route: Route, params: &[&dyn Display]) -> String {
        route.url(&self.base, params)
    }

    /// Builds the request (URL from `route`/`params` unless the caller already added a query)
    pub(crate) fn request<B: Serialize>(
        &self,
        method: Method,
        url: String,
        body: Body<'_, B>,
        reason: Option<&str>,
    ) -> Result<Request> {
        let mut headers = Vec::with_capacity(4);
        headers.push(("User-Agent", Cow::Borrowed(USER_AGENT)));
        if let Some(authorization) = &self.authorization {
            headers.push(("Authorization", Cow::Owned(authorization.clone())));
        }
        if let Some(reason) = reason {
            headers.push(("X-Audit-Log-Reason", Cow::Owned(encode_reason(reason))));
        }
        let body = match body {
            Body::None => None,
            Body::Json(body) => {
                headers.push(("Content-Type", Cow::Borrowed("application/json")));
                Some(serde_json::to_vec(body).map_err(Error::Encode)?)
            }
            Body::WithFiles(body, []) => {
                headers.push(("Content-Type", Cow::Borrowed("application/json")));
                Some(serde_json::to_vec(body).map_err(Error::Encode)?)
            }
            Body::WithFiles(body, files) => {
                let (bytes, content_type) = multipart::encode(body, files)?;
                headers.push(("Content-Type", Cow::Owned(content_type)));
                Some(bytes)
            }
            Body::Form(fields, file) => {
                let (bytes, content_type) = multipart::encode_form(fields, file);
                headers.push(("Content-Type", Cow::Owned(content_type)));
                Some(bytes)
            }
        };
        Ok(Request {
            method,
            url,
            headers,
            body,
        })
    }

    /// Sends, waiting out 429s no longer than the configured cap, and maps error statuses
    pub(crate) async fn send(&self, request: &Request) -> Result<RawResponse> {
        let mut attempt = 0;
        loop {
            let response = self.transport.send(request).await?;
            debug!(status = response.status, "response");
            match response.status {
                429 => {
                    let limited = rate_limited(&response);
                    if attempt >= MAX_RATE_LIMIT_RETRIES
                        || limited.retry_after > self.max_rate_limit_wait.as_secs_f64()
                    {
                        warn!(
                            retry_after = limited.retry_after,
                            global = limited.global,
                            bucket = limited.bucket.as_deref(),
                            attempt,
                            "rate limited; giving up",
                        );
                        return Err(Error::RateLimited(limited));
                    }
                    attempt += 1;
                    debug!(
                        retry_after = limited.retry_after,
                        global = limited.global,
                        bucket = limited.bucket.as_deref(),
                        attempt,
                        "rate limited; waiting",
                    );
                    let wait = Duration::from_secs_f64(limited.retry_after.max(0.0));
                    self.transport.sleep(wait).await;
                }
                401 => {
                    return Err(Error::Unauthorized(ApiError::parse(401, &response.body)));
                }
                _ if response.is_success() => return Ok(response),
                status => {
                    return Err(Error::Api(
                        ApiError::parse(status, &response.body)
                            .unwrap_or_else(|| ApiError::unparsed(status, &response.body)),
                    ));
                }
            }
        }
    }

    pub(crate) async fn call<B: Serialize, R>(
        &self,
        route: Route,
        url: String,
        body: Body<'_, B>,
        reason: Option<&str>,
    ) -> Result<Response<R>> {
        let request = self.request(route.method, url, body, reason)?;
        self.send(&request)
            .instrument(span(route))
            .await
            .map(Response::new)
    }

    /// For 204 Endpoints
    pub(crate) async fn call_empty<B: Serialize>(
        &self,
        route: Route,
        url: String,
        body: Body<'_, B>,
        reason: Option<&str>,
    ) -> Result<()> {
        let request = self.request(route.method, url, body, reason)?;
        self.send(&request).instrument(span(route)).await.map(drop)
    }
}

/// The `rest` span: the route template, not the URL (Interaction Endpoints carry the token)
#[cfg_attr(not(feature = "tracing"), allow(unused_variables))]
fn span(route: Route) -> Span {
    debug_span!("rest", method = route.method.as_str(), route = route.path)
}

/// Audit log reasons are sent URL-encoded so non-ASCII survives the header
fn encode_reason(reason: &str) -> String {
    let mut encoded = String::with_capacity(reason.len());
    for byte in reason.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b' ' => {
                encoded.push(byte as char)
            }
            _ => {
                let _ = write!(encoded, "%{byte:02X}");
            }
        }
    }
    encoded
}

#[derive(Deserialize)]
struct RateLimitedBody {
    retry_after: f64,
    #[serde(default)]
    global: bool,
}

/// From the 429 body, falling back to the `Retry-After` header (Cloudflare-level limits send HTML)
fn rate_limited(response: &RawResponse) -> RateLimited {
    let bucket = response.header("x-ratelimit-bucket").map(str::to_string);
    match serde_json::from_slice::<RateLimitedBody>(&response.body) {
        Ok(body) => RateLimited {
            retry_after: body.retry_after,
            global: body.global || response.header("x-ratelimit-global").is_some(),
            bucket,
        },
        Err(_) => RateLimited {
            retry_after: response
                .header("retry-after")
                .and_then(|s| s.parse().ok())
                .unwrap_or(f64::INFINITY),
            global: true,
            bucket,
        },
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use super::super::{JsonErrorCode, testing::MockTransport};
    use super::*;

    const ROUTE: Route = Route {
        method: Method::Get,
        path: "/x",
    };

    fn get(rest: &Rest<MockTransport>) -> Result<Response<serde_json::Value>> {
        block_on(rest.call(ROUTE, rest.url(ROUTE, &[]), Body::NONE, None))
    }

    #[test]
    fn sends_auth_user_agent_and_reason() {
        let rest = Rest::new(MockTransport::default(), Some("Bot t".into()));
        let request = rest
            .request(Method::Post, "u".into(), Body::Json(&1), Some("bad bot ✓"))
            .unwrap();
        assert_eq!(request.header("authorization"), Some("Bot t"));
        assert!(
            request
                .header("user-agent")
                .unwrap()
                .starts_with("DiscordBot (")
        );
        assert_eq!(request.header("content-type"), Some("application/json"));
        assert_eq!(
            request.header("x-audit-log-reason"),
            Some("bad bot %E2%9C%93")
        );
        assert_eq!(request.body.as_deref(), Some(&b"1"[..]));
    }

    #[test]
    fn waits_out_short_rate_limits() {
        let transport = MockTransport::default()
            .respond(429, r#"{"retry_after":0.5,"global":false,"message":""}"#)
            .respond(200, r#"{"ok":true}"#);
        let rest = Rest::new(transport, None);
        let response = get(&rest).unwrap();
        assert_eq!(response.model().unwrap()["ok"], true);
        assert_eq!(rest.transport().sleeps(), [Duration::from_millis(500)]);
    }

    #[test]
    fn long_rate_limits_are_errors() {
        let transport =
            MockTransport::default().respond(429, r#"{"retry_after":30,"global":true}"#);
        let rest = Rest::new(transport, None);
        match get(&rest) {
            Err(Error::RateLimited(limited)) => {
                assert_eq!(limited.retry_after, 30.0);
                assert!(limited.global);
            }
            other => panic!("{other:?}"),
        }
        assert!(rest.transport().sleeps().is_empty());
    }

    #[test]
    fn gives_up_after_retries() {
        let limited = r#"{"retry_after":0.1}"#;
        let transport = MockTransport::default()
            .respond(429, limited)
            .respond(429, limited)
            .respond(429, limited)
            .respond(429, limited);
        let rest = Rest::new(transport, None);
        assert!(matches!(get(&rest), Err(Error::RateLimited(_))));
        assert_eq!(rest.transport().requests().len(), 4);
    }

    #[test]
    fn non_json_rate_limit_uses_retry_after_header() {
        let transport = MockTransport::default().respond_raw(RawResponse {
            status: 429,
            headers: vec![("Retry-After".into(), "1".into())],
            body: b"<html>".to_vec(),
        });
        let rest = Rest::new(transport, None);
        get(&rest).unwrap();
        assert_eq!(rest.transport().sleeps(), [Duration::from_secs(1)]);
    }

    #[test]
    fn maps_error_statuses() {
        let transport = MockTransport::default()
            .respond(401, r#"{"code":0,"message":"401: Unauthorized"}"#)
            .respond(403, r#"{"code":50013,"message":"Missing Permissions"}"#)
            .respond(502, "bad gateway");
        let rest = Rest::new(transport, None);
        assert!(matches!(get(&rest), Err(Error::Unauthorized(Some(_)))));
        assert_eq!(
            get(&rest).unwrap_err().code(),
            Some(JsonErrorCode::MissingPermissions)
        );
        match get(&rest) {
            Err(Error::Api(e)) => assert_eq!((e.status, e.message.as_str()), (502, "bad gateway")),
            other => panic!("{other:?}"),
        }
    }
}
