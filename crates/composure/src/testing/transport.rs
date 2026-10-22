use std::{
    collections::VecDeque,
    fmt,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use serde_json::Value;

use crate::rest::{DISCORD_API, Method, RawResponse, Request, Transport, TransportError};

type Answer = Box<dyn Fn(&Request) -> Option<RawResponse> + Send + Sync>;

/// A [`Transport`] that records every request and answers it: first from the queued responses
/// ([`respond`](Self::respond)), in order, then from the [`answering`](Self::answering) function,
/// else with 200 `{}`. Rate-limit waits are recorded, not slept.
#[derive(Default)]
pub struct MockTransport {
    requests: Mutex<Vec<Request>>,
    responses: Mutex<VecDeque<RawResponse>>,
    sleeps: Mutex<Vec<Duration>>,
    answer: Option<Answer>,
}

impl fmt::Debug for MockTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MockTransport")
            .field("requests", &lock(&self.requests).len())
            .field("queued", &lock(&self.responses).len())
            .finish_non_exhaustive()
    }
}

/// A response with `body` as JSON, for [`MockTransport::answering`]
pub fn json_response(status: u16, body: &Value) -> RawResponse {
    RawResponse {
        status,
        headers: vec![("content-type".into(), "application/json".into())],
        body: serde_json::to_vec(body).expect("JSON values serialize"),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl MockTransport {
    /// Answers every request with 200 `{}`
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `body` with `status` for the next request not answered yet
    pub fn respond(self, status: u16, body: &str) -> Self {
        self.respond_raw(RawResponse {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        })
    }

    /// Queues `body` as JSON with `status`
    pub fn respond_json(self, status: u16, body: &Value) -> Self {
        self.respond_raw(json_response(status, body))
    }

    /// Queues a whole response, headers included
    pub fn respond_raw(self, response: RawResponse) -> Self {
        self.queue(response);
        self
    }

    /// Queues `response` once the Transport is in use (as a [`TestPlatform`](super::TestPlatform)'s)
    pub fn queue(&self, response: RawResponse) {
        lock(&self.responses).push_back(response);
    }

    /// Answers requests the queue doesn't with `answer`, when it returns `Some`: canned bodies by
    /// route, or a stand-in for another API the bot calls
    ///
    /// ```
    /// use composure::{rest::Method, testing::{MockTransport, json_response}};
    /// use serde_json::json;
    ///
    /// let transport = MockTransport::new().answering(|request| {
    ///     (request.method == Method::Get && request.url.ends_with("/users/@me"))
    ///         .then(|| json_response(200, &json!({ "id": "9", "username": "bot", "discriminator": "0" })))
    /// });
    /// # let _ = transport;
    /// ```
    pub fn answering(
        mut self,
        answer: impl Fn(&Request) -> Option<RawResponse> + Send + Sync + 'static,
    ) -> Self {
        self.answer = Some(Box::new(answer));
        self
    }

    /// Every request sent so far
    pub fn requests(&self) -> Vec<Request> {
        lock(&self.requests).clone()
    }

    /// The `index`th request sent
    pub fn request(&self, index: usize) -> Request {
        lock(&self.requests)[index].clone()
    }

    /// (method, path) of every request sent: the path after Discord's API version
    /// (`/channels/1/messages`), or the whole URL for other hosts
    pub fn sent(&self) -> Vec<(Method, String)> {
        lock(&self.requests)
            .iter()
            .map(|request| (request.method, path(&request.url).to_string()))
            .collect()
    }

    /// The paths of [`sent`](Self::sent)
    pub fn paths(&self) -> Vec<String> {
        self.sent().into_iter().map(|(_, path)| path).collect()
    }

    /// The `index`th request's body as JSON; panics if it has none or it is not JSON
    pub fn body(&self, index: usize) -> Value {
        let requests = lock(&self.requests);
        let body = requests[index]
            .body
            .as_deref()
            .expect("the request has a body");
        serde_json::from_slice(body).expect("the request body is JSON")
    }

    /// The `index`th request's body as text (multipart bodies included); empty without one
    pub fn text(&self, index: usize) -> String {
        let requests = lock(&self.requests);
        String::from_utf8_lossy(requests[index].body.as_deref().unwrap_or_default()).into_owned()
    }

    /// Every rate-limit wait the client asked for
    pub fn sleeps(&self) -> Vec<Duration> {
        lock(&self.sleeps).clone()
    }

    /// Forgets the requests sent so far, to check only what comes next
    pub fn clear(&self) {
        lock(&self.requests).clear();
    }
}

/// The part of `url` after Discord's API version, or `url`
fn path(url: &str) -> &str {
    url.strip_prefix(DISCORD_API).unwrap_or(url)
}

impl Transport for MockTransport {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        lock(&self.requests).push(request.clone());
        let queued = lock(&self.responses).pop_front();
        Ok(queued
            .or_else(|| self.answer.as_ref().and_then(|answer| answer(request)))
            .unwrap_or_else(|| RawResponse {
                status: 200,
                headers: Vec::new(),
                body: b"{}".to_vec(),
            }))
    }

    async fn sleep(&self, duration: Duration) {
        lock(&self.sleeps).push(duration);
    }
}
