use ed25519_dalek::{PublicKey, Signature, SignatureError, Verifier};
use hex::FromHexError;

pub enum ValidateError {
    HexError(FromHexError),
    SignatureError(SignatureError),
}

/// Validates a request using ed25519
pub fn validate_request(
    public_key: &str,
    signature: &str,
    timestamp: &str,
    body: &[u8],
) -> Result<(), ValidateError> {
    let public_key = hex::decode(public_key).map_err(|e| ValidateError::HexError(e))?;
    let signature = hex::decode(signature).map_err(|e| ValidateError::HexError(e))?;
    validate_bytes(
        public_key.as_slice(),
        signature.as_slice(),
        timestamp.as_bytes(),
        body,
    )
    .map_err(|e| ValidateError::SignatureError(e))
}

/// Validates the request using a public key, signature, timestamp, and body as bytes
fn validate_bytes(
    public_key: &[u8],
    signature: &[u8],
    timestamp: &[u8],
    body: &[u8],
) -> Result<(), SignatureError> {
    let message = [timestamp, body].concat();

    let public_key = PublicKey::from_bytes(&public_key)?;
    let signature = Signature::from_bytes(&signature)?;

    public_key.verify(&message, &signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn validate_request_ok() {
        let public_key = "852aec10972ef6dd0431747902c779342cc411ad6d42c2de16ef4c87895c61ad";
        let sig = "c91641b5c3d12f9c819d9b5c568ef7d660e7f9abc2c312f296c562f6d7b028dac80c6c8e5c8a11f7a21ee28dbb8c6cf2762118bee45c00b2df78065b3b59f20c";
        let timestamp = "1682372142";
        let body = br#"{"app_permissions":"137411140374081","application_id":"1052322265397739523","channel":{"flags":0,"guild_id":"798662131062931547","id":"941169456686723122","last_message_id":"1100155827400229026","name":"bot-stuff","nsfw":false,"parent_id":"798662131678969866","permissions":"140737488355327","position":1,"rate_limit_per_user":0,"topic":null,"type":0},"channel_id":"941169456686723122","data":{"guild_id":"798662131062931547","id":"1052358444704862218","name":"ping","type":1},"entitlement_sku_ids":[],"entitlements":[],"guild_id":"798662131062931547","guild_locale":"en-US","id":"1100173248714518568","locale":"en-US","member":{"avatar":null,"communication_disabled_until":null,"deaf":false,"flags":0,"is_pending":false,"joined_at":"2021-01-12T21:18:10.481000+00:00","mute":false,"nick":null,"pending":false,"permissions":"140737488355327","premium_since":null,"roles":["943607715639484456"],"user":{"avatar":"fa82e15e24ee16c9fcbf8dd34d10b4cc","avatar_decoration":null,"discriminator":"9846","display_name":null,"global_name":null,"id":"282265607313817601","public_flags":0,"username":"BlueFrog"}},"token":"aW50ZXJhY3Rpb246MTEwMDE3MzI0ODcxNDUxODU2ODppVTFuSkNSbndrZ01Na3RCWk81MVhTWkdSbk8yTlBaM1U3Z3JlckR4YUZJMTZFTm9wc21nZnlaSnN4ZUZCTTd0Q0Jzc09ac3BHV1E1MGlBZGZnZzh0NDJmTElIcTB1M0FZQTJPS1BxcG1GTEtZUjNDWWFEamhEeTRPMWZnS0R4dQ","type":2,"version":1}"#;

        let res = validate_request(public_key, sig, timestamp, body);

        assert!(res.is_ok());
    }

    #[test]
    pub fn validate_request_err() {
        let public_key = "852aec10972ef6dd0431747902c779342cc411ad6d42c2de16ef4c87895c61ad";
        let sig = "c91641b5c3d12f9c819d9b5c568ef7d660e7f9abc2c312f296c562f6d7b028dac80c6c8e5c8a11f7a21ee28dbb8c6cf2762118bee45c00b2df78065b3b59f20c";
        let timestamp = "1682371237";
        let body = br#"{"app_permissions":"137411140374081","application_id":"1052322265397739523","channel":{"flags":0,"guild_id":"798662131062931547","id":"941169456686723122","last_message_id":"1100155827400229026","name":"bot-stuff","nsfw":false,"parent_id":"798662131678969866","permissions":"140737488355327","position":1,"rate_limit_per_user":0,"topic":null,"type":0},"channel_id":"941169456686723122","data":{"guild_id":"798662131062931547","id":"1052358444704862218","name":"ping","type":1},"entitlement_sku_ids":[],"entitlements":[],"guild_id":"798662131062931547","guild_locale":"en-US","id":"1100173248714518568","locale":"en-US","member":{"avatar":null,"communication_disabled_until":null,"deaf":false,"flags":0,"is_pending":false,"joined_at":"2021-01-12T21:18:10.481000+00:00","mute":false,"nick":null,"pending":false,"permissions":"140737488355327","premium_since":null,"roles":["943607715639484456"],"user":{"avatar":"fa82e15e24ee16c9fcbf8dd34d10b4cc","avatar_decoration":null,"discriminator":"9846","display_name":null,"global_name":null,"id":"282265607313817601","public_flags":0,"username":"BlueFrog"}},"token":"aW50ZXJhY3Rpb246MTEwMDE3MzI0ODcxNDUxODU2ODppVTFuSkNSbndrZ01Na3RCWk81MVhTWkdSbk8yTlBaM1U3Z3JlckR4YUZJMTZFTm9wc21nZnlaSnN4ZUZCTTd0Q0Jzc09ac3BHV1E1MGlBZGZnZzh0NDJmTElIcTB1M0FZQTJPS1BxcG1GTEtZUjNDWWFEamhEeTRPMWZnS0R4dQ","type":2,"version":1}"#;

        let res = validate_request(public_key, sig, timestamp, body);

        assert!(res.is_err());
    }
}
