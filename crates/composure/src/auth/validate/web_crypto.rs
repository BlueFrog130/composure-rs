use js_sys::{Array, Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use super::ValidateError;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["crypto", "subtle"], js_name = importKey, catch)]
    fn import_key(
        format: &str,
        key_data: &Uint8Array,
        algorithm: &str,
        extractable: bool,
        key_usages: &Array,
    ) -> Result<Promise, JsValue>;

    #[wasm_bindgen(js_namespace = ["crypto", "subtle"], js_name = verify, catch)]
    fn subtle_verify(
        algorithm: &str,
        key: &JsValue,
        signature: &Uint8Array,
        data: &Uint8Array,
    ) -> Result<Promise, JsValue>;
}

const ALGORITHM: &str = "Ed25519";

/// Verifies with the host's `crypto.subtle`
pub(super) async fn verify(
    public_key: &[u8; 32],
    signature: &[u8; 64],
    timestamp: &[u8],
    body: &[u8],
) -> Result<(), ValidateError> {
    let usages = Array::of1(&JsValue::from_str("verify"));
    let key = import_key(
        "raw",
        &Uint8Array::from(&public_key[..]),
        ALGORITHM,
        false,
        &usages,
    )
    .map_err(|_| ValidateError::InvalidPublicKey)?;
    let key = JsFuture::from(key)
        .await
        .map_err(|_| ValidateError::InvalidPublicKey)?;

    // Web Crypto needs one contiguous message. Build it directly in JS memory, so the body is
    // copied once and nothing is allocated on the wasm heap.
    let split = timestamp.len() as u32;
    let message = Uint8Array::new_with_length(split + body.len() as u32);
    message.subarray(0, split).copy_from(timestamp);
    message.subarray(split, message.length()).copy_from(body);

    let valid = subtle_verify(ALGORITHM, &key, &Uint8Array::from(&signature[..]), &message)
        .map_err(|_| ValidateError::InvalidSignature)?;
    let valid = JsFuture::from(valid)
        .await
        .map_err(|_| ValidateError::InvalidSignature)?;

    if valid.is_truthy() {
        Ok(())
    } else {
        Err(ValidateError::InvalidSignature)
    }
}
