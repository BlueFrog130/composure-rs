use std::{
    collections::VecDeque,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use super::{Method, RawResponse, Request, Transport, TransportError};

/// Records requests and answers them from a queue (200 `{}` once empty)
#[derive(Default)]
pub(crate) struct MockTransport {
    pub(crate) requests: Recorded<Vec<Request>>,
    pub(crate) responses: Recorded<VecDeque<RawResponse>>,
    pub(crate) sleeps: Recorded<Vec<Duration>>,
}

/// A `RefCell`-like `Mutex`, so `MockTransport` is `Sync` like every Transport
#[derive(Default)]
pub(crate) struct Recorded<T>(Mutex<T>);

impl<T> Recorded<T> {
    pub(crate) fn borrow(&self) -> MutexGuard<'_, T> {
        self.0.lock().unwrap()
    }

    pub(crate) fn borrow_mut(&self) -> MutexGuard<'_, T> {
        self.0.lock().unwrap()
    }
}

impl MockTransport {
    pub(crate) fn respond(self, status: u16, body: &str) -> Self {
        self.responses.borrow_mut().push_back(RawResponse {
            status,
            headers: Vec::new(),
            body: body.as_bytes().to_vec(),
        });
        self
    }

    pub(crate) fn request(&self, index: usize) -> Request {
        self.requests.borrow()[index].clone()
    }

    /// (method, path after the API version) of every request sent
    pub(crate) fn sent(&self) -> Vec<(Method, String)> {
        self.requests
            .borrow()
            .iter()
            .map(|r| (r.method, r.url.split_once("/v10").unwrap().1.to_string()))
            .collect()
    }

    pub(crate) fn body(&self, index: usize) -> serde_json::Value {
        serde_json::from_slice(self.requests.borrow()[index].body.as_deref().unwrap()).unwrap()
    }
}

impl Transport for MockTransport {
    async fn send(&self, request: &Request) -> Result<RawResponse, TransportError> {
        self.requests.borrow_mut().push(request.clone());
        Ok(self
            .responses
            .borrow_mut()
            .pop_front()
            .unwrap_or(RawResponse {
                status: 200,
                headers: Vec::new(),
                body: b"{}".to_vec(),
            }))
    }

    async fn sleep(&self, duration: Duration) {
        self.sleeps.borrow_mut().push(duration);
    }
}
