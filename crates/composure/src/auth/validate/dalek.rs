use ed25519_dalek::{Signature, VerifyingKey, ed25519::signature::MultipartVerifier};

use super::ValidateError;

/// Hashes `timestamp` and `body` as separate parts, so the message is never concatenated
pub(super) async fn verify(
    public_key: &[u8; 32],
    signature: &[u8; 64],
    timestamp: &[u8],
    body: &[u8],
) -> Result<(), ValidateError> {
    let public_key =
        VerifyingKey::from_bytes(public_key).map_err(|_| ValidateError::InvalidPublicKey)?;
    let signature = Signature::from_bytes(signature);

    public_key
        .multipart_verify(&[timestamp, body], &signature)
        .map_err(|_| ValidateError::InvalidSignature)
}
