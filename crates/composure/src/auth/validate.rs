//! Discord request signature validation (ed25519 over `timestamp || body`).
//!
//! Uses `ed25519-dalek` by default. With the `web-crypto` feature on `wasm32-unknown-unknown`
//! (JS hosts with Ed25519 Web Crypto, such as Cloudflare Workers) it uses the host's
//! `crypto.subtle` instead, so dalek is left out of the bundle. Neither path copies the body into
//! a new wasm/heap buffer.

use hex::FromHexError;

#[cfg(not(all(feature = "web-crypto", target_arch = "wasm32", target_os = "unknown")))]
mod dalek;
#[cfg(not(all(feature = "web-crypto", target_arch = "wasm32", target_os = "unknown")))]
use dalek as backend;

#[cfg(all(feature = "web-crypto", target_arch = "wasm32", target_os = "unknown"))]
mod web_crypto;
#[cfg(all(feature = "web-crypto", target_arch = "wasm32", target_os = "unknown"))]
use web_crypto as backend;

#[derive(Debug)]
pub enum ValidateError {
    /// Public key or signature is not hex of the right length (32 / 64 bytes)
    HexError(FromHexError),
    /// Public key is not a valid ed25519 key (or the host can't import it)
    InvalidPublicKey,
    /// Signature does not match
    InvalidSignature,
}

/// The application's public key, decoded (and with `web-crypto`, imported into the host) once so
/// it can verify any number of requests. Adapters keep one for the life of the isolate.
#[derive(Clone)]
pub struct PublicKey(backend::Key);

impl PublicKey {
    /// Parses the hex public key from the Developer Portal
    pub async fn from_hex(public_key: &str) -> Result<Self, ValidateError> {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(public_key, &mut bytes).map_err(ValidateError::HexError)?;
        backend::Key::new(&bytes).await.map(Self)
    }

    /// Checks the `X-Signature-Ed25519` signature (hex) of `timestamp || body`
    pub async fn verify(
        &self,
        signature: &str,
        timestamp: &str,
        body: &[u8],
    ) -> Result<(), ValidateError> {
        let mut signature_bytes = [0u8; 64];
        hex::decode_to_slice(signature, &mut signature_bytes).map_err(ValidateError::HexError)?;
        self.0
            .verify(&signature_bytes, timestamp.as_bytes(), body)
            .await
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PublicKey")
    }
}

/// Validates a request using ed25519. Parses `public_key` on every call; keep a [`PublicKey`] to
/// verify many requests.
///
/// Async because Web Crypto is; with dalek it completes on first poll.
pub async fn validate_request(
    public_key: &str,
    signature: &str,
    timestamp: &str,
    body: &[u8],
) -> Result<(), ValidateError> {
    PublicKey::from_hex(public_key)
        .await?
        .verify(signature, timestamp, body)
        .await
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use super::*;

    fn validate(
        public_key: &str,
        signature: &str,
        timestamp: &str,
        body: &[u8],
    ) -> Result<(), ValidateError> {
        block_on(validate_request(public_key, signature, timestamp, body))
    }

    #[test]
    pub fn validate_request_ok() {
        let public_key = "852aec10972ef6dd0431747902c779342cc411ad6d42c2de16ef4c87895c61ad";
        let sig = "c91641b5c3d12f9c819d9b5c568ef7d660e7f9abc2c312f296c562f6d7b028dac80c6c8e5c8a11f7a21ee28dbb8c6cf2762118bee45c00b2df78065b3b59f20c";
        let timestamp = "1682372142";
        let body = br#"{"app_permissions":"137411140374081","application_id":"1052322265397739523","channel":{"flags":0,"guild_id":"798662131062931547","id":"941169456686723122","last_message_id":"1100155827400229026","name":"bot-stuff","nsfw":false,"parent_id":"798662131678969866","permissions":"140737488355327","position":1,"rate_limit_per_user":0,"topic":null,"type":0},"channel_id":"941169456686723122","data":{"guild_id":"798662131062931547","id":"1052358444704862218","name":"ping","type":1},"entitlement_sku_ids":[],"entitlements":[],"guild_id":"798662131062931547","guild_locale":"en-US","id":"1100173248714518568","locale":"en-US","member":{"avatar":null,"communication_disabled_until":null,"deaf":false,"flags":0,"is_pending":false,"joined_at":"2021-01-12T21:18:10.481000+00:00","mute":false,"nick":null,"pending":false,"permissions":"140737488355327","premium_since":null,"roles":["943607715639484456"],"user":{"avatar":"fa82e15e24ee16c9fcbf8dd34d10b4cc","avatar_decoration":null,"discriminator":"9846","display_name":null,"global_name":null,"id":"282265607313817601","public_flags":0,"username":"BlueFrog"}},"token":"aW50ZXJhY3Rpb246MTEwMDE3MzI0ODcxNDUxODU2ODppVTFuSkNSbndrZ01Na3RCWk81MVhTWkdSbk8yTlBaM1U3Z3JlckR4YUZJMTZFTm9wc21nZnlaSnN4ZUZCTTd0Q0Jzc09ac3BHV1E1MGlBZGZnZzh0NDJmTElIcTB1M0FZQTJPS1BxcG1GTEtZUjNDWWFEamhEeTRPMWZnS0R4dQ","type":2,"version":1}"#;

        let res = validate(public_key, sig, timestamp, body);

        assert!(res.is_ok());
    }

    #[test]
    pub fn validate_request_err() {
        let public_key = "852aec10972ef6dd0431747902c779342cc411ad6d42c2de16ef4c87895c61ad";
        let sig = "c91641b5c3d12f9c819d9b5c568ef7d660e7f9abc2c312f296c562f6d7b028dac80c6c8e5c8a11f7a21ee28dbb8c6cf2762118bee45c00b2df78065b3b59f20c";
        let timestamp = "1682371237";
        let body = br#"{"app_permissions":"137411140374081","application_id":"1052322265397739523","channel":{"flags":0,"guild_id":"798662131062931547","id":"941169456686723122","last_message_id":"1100155827400229026","name":"bot-stuff","nsfw":false,"parent_id":"798662131678969866","permissions":"140737488355327","position":1,"rate_limit_per_user":0,"topic":null,"type":0},"channel_id":"941169456686723122","data":{"guild_id":"798662131062931547","id":"1052358444704862218","name":"ping","type":1},"entitlement_sku_ids":[],"entitlements":[],"guild_id":"798662131062931547","guild_locale":"en-US","id":"1100173248714518568","locale":"en-US","member":{"avatar":null,"communication_disabled_until":null,"deaf":false,"flags":0,"is_pending":false,"joined_at":"2021-01-12T21:18:10.481000+00:00","mute":false,"nick":null,"pending":false,"permissions":"140737488355327","premium_since":null,"roles":["943607715639484456"],"user":{"avatar":"fa82e15e24ee16c9fcbf8dd34d10b4cc","avatar_decoration":null,"discriminator":"9846","display_name":null,"global_name":null,"id":"282265607313817601","public_flags":0,"username":"BlueFrog"}},"token":"aW50ZXJhY3Rpb246MTEwMDE3MzI0ODcxNDUxODU2ODppVTFuSkNSbndrZ01Na3RCWk81MVhTWkdSbk8yTlBaM1U3Z3JlckR4YUZJMTZFTm9wc21nZnlaSnN4ZUZCTTd0Q0Jzc09ac3BHV1E1MGlBZGZnZzh0NDJmTElIcTB1M0FZQTJPS1BxcG1GTEtZUjNDWWFEamhEeTRPMWZnS0R4dQ","type":2,"version":1}"#;

        let res = validate(public_key, sig, timestamp, body);

        assert!(matches!(res, Err(ValidateError::InvalidSignature)));
    }

    const PUBLIC_KEY: &str = "852aec10972ef6dd0431747902c779342cc411ad6d42c2de16ef4c87895c61ad";
    const SIGNATURE: &str = "c91641b5c3d12f9c819d9b5c568ef7d660e7f9abc2c312f296c562f6d7b028dac80c6c8e5c8a11f7a21ee28dbb8c6cf2762118bee45c00b2df78065b3b59f20c";

    #[test]
    pub fn validate_request_malformed_hex() {
        let res = validate("not hex", SIGNATURE, "1682372142", b"{}");
        assert!(matches!(res, Err(ValidateError::HexError(_))));

        let res = validate(PUBLIC_KEY, "zz", "1682372142", b"{}");
        assert!(matches!(res, Err(ValidateError::HexError(_))));
    }

    #[test]
    pub fn validate_request_wrong_length_public_key() {
        let res = validate(&PUBLIC_KEY[..62], SIGNATURE, "1682372142", b"{}");
        assert!(matches!(res, Err(ValidateError::HexError(_))));
    }

    #[test]
    pub fn validate_request_wrong_length_signature() {
        let res = validate(PUBLIC_KEY, &SIGNATURE[..126], "1682372142", b"{}");
        assert!(matches!(res, Err(ValidateError::HexError(_))));
    }
}
