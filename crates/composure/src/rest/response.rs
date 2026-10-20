use std::marker::PhantomData;

use serde::de::DeserializeOwned;

use super::{Error, RawResponse, Result};

/// A successful response whose body is decoded only when asked for, so callers that ignore it
/// (fire-and-forget edits, ...) skip the parse.
#[derive(Debug, Clone)]
pub struct Response<T> {
    raw: RawResponse,
    _model: PhantomData<fn() -> T>,
}

impl<T> Response<T> {
    pub(crate) fn new(raw: RawResponse) -> Self {
        Self {
            raw,
            _model: PhantomData,
        }
    }

    /// The HTTP status
    pub fn status(&self) -> u16 {
        self.raw.status
    }

    /// The value of header `name`, matched case-insensitively
    pub fn header(&self, name: &str) -> Option<&str> {
        self.raw.header(name)
    }

    /// The raw body
    pub fn bytes(&self) -> &[u8] {
        &self.raw.body
    }

    /// The underlying response
    pub fn into_raw(self) -> RawResponse {
        self.raw
    }
}

impl<T: DeserializeOwned> Response<T> {
    /// The body decoded as `T`
    pub fn model(&self) -> Result<T> {
        serde_json::from_slice(&self.raw.body).map_err(Error::Decode)
    }
}
