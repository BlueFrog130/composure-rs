use ed25519_dalek::{Signer, SigningKey};

use crate::adapter::{SIGNATURE_HEADER, TIMESTAMP_HEADER};

/// An Ed25519 key standing in for Discord's: give its [`public_key`](Self::public_key) to the
/// Adapter (or [`handle_interaction`](crate::adapter::handle_interaction)) and sign request bodies
/// with it
///
/// ```
/// use composure::{
///     adapter::handle_interaction,
///     command::NoHandler,
///     testing::{TestKey, block_on},
/// };
///
/// let key = TestKey::default();
/// let request = key.signed(br#"{"id":"2","application_id":"1","type":1,"token":"t","version":1}"#);
/// let outcome = block_on(handle_interaction(
///     &key.public_key(), Some(&request.signature), Some(&request.timestamp), &request.body,
///     &NoHandler, &(),
/// ));
/// assert_eq!(outcome.status(), 200); // a Ping, answered with a Pong
///
/// let forged = block_on(handle_interaction(
///     &key.public_key(), Some(&request.signature), Some("1700000001"), &request.body,
///     &NoHandler, &(),
/// ));
/// assert_eq!(forged.status(), 401);
/// ```
#[derive(Debug)]
pub struct TestKey(SigningKey);

/// A body with the signature headers Discord would send with it
#[derive(Debug, Clone)]
pub struct SignedRequest {
    /// The [`SIGNATURE_HEADER`] value: the hex signature of `timestamp` + `body`
    pub signature: String,
    /// The [`TIMESTAMP_HEADER`] value
    pub timestamp: String,
    /// The request body
    pub body: Vec<u8>,
}

impl SignedRequest {
    /// (name, value) of both signature headers
    pub fn headers(&self) -> [(&'static str, &str); 2] {
        [
            (SIGNATURE_HEADER, &self.signature),
            (TIMESTAMP_HEADER, &self.timestamp),
        ]
    }
}

/// The timestamp [`TestKey::signed`] signs with
const TIMESTAMP: &str = "1760000000";

impl TestKey {
    /// The key derived from `seed`
    pub fn new(seed: [u8; 32]) -> Self {
        Self(SigningKey::from_bytes(&seed))
    }

    /// The public key as hex, as the Developer Portal shows it (`DISCORD_PUBLIC_KEY`)
    pub fn public_key(&self) -> String {
        hex::encode(self.0.verifying_key().as_bytes())
    }

    /// The hex signature of `timestamp` followed by `body`
    pub fn sign(&self, timestamp: &str, body: &[u8]) -> String {
        let message = [timestamp.as_bytes(), body].concat();
        hex::encode(self.0.sign(&message).to_bytes())
    }

    /// `body`, signed at a fixed timestamp
    pub fn signed(&self, body: impl Into<Vec<u8>>) -> SignedRequest {
        let body = body.into();
        SignedRequest {
            signature: self.sign(TIMESTAMP, &body),
            timestamp: TIMESTAMP.into(),
            body,
        }
    }
}

/// The key from seed `[7; 32]`
impl Default for TestKey {
    fn default() -> Self {
        Self::new([7; 32])
    }
}
