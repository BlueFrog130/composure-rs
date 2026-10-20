use ed25519_dalek::{Signature, VerifyingKey, ed25519::signature::MultipartVerifier};

use super::ValidateError;

/// A decompressed public key, so the curve point is decoded once rather than per request
#[derive(Clone)]
pub(super) struct Key(VerifyingKey);

impl Key {
    pub(super) async fn new(public_key: &[u8; 32]) -> Result<Self, ValidateError> {
        VerifyingKey::from_bytes(public_key)
            .map(Self)
            .map_err(|_| ValidateError::InvalidPublicKey)
    }

    /// Hashes `timestamp` and `body` as separate parts, so the message is never concatenated
    pub(super) async fn verify(
        &self,
        signature: &[u8; 64],
        timestamp: &[u8],
        body: &[u8],
    ) -> Result<(), ValidateError> {
        self.0
            .multipart_verify(&[timestamp, body], &Signature::from_bytes(signature))
            .map_err(|_| ValidateError::InvalidSignature)
    }
}
