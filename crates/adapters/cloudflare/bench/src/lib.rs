//! Request validation benchmark worker.
//!
//! Runs [`composure::auth::validate_request`] `X-Iterations` times on the request and answers with
//! how many passed. With `X-Cached-Key: 1` the key is parsed once and only
//! [`PublicKey::verify`](composure::auth::PublicKey::verify) is repeated, as the adapter does.
//! Workers freeze timers while code runs, so timing happens on the client; see
//! `tests/wrangler.rs`. Built with ed25519-dalek by default, Web Crypto with `web-crypto`.

use composure::auth::PublicKey;
use worker::{Context, Env, Request, Response, event};

#[event(fetch)]
async fn fetch(mut req: Request, _env: Env, _ctx: Context) -> worker::Result<Response> {
    let header = |name: &str| req.headers().get(name).ok().flatten().unwrap_or_default();
    let public_key = header("X-Public-Key");
    let signature = header("X-Signature-Ed25519");
    let timestamp = header("X-Signature-Timestamp");
    let iterations: u32 = header("X-Iterations").parse().unwrap_or(1);
    let cached = header("X-Cached-Key") == "1";
    let body = req.bytes().await?;

    let mut valid = 0u32;
    if cached {
        let Ok(key) = PublicKey::from_hex(&public_key).await else {
            return Response::ok("0");
        };
        for _ in 0..iterations {
            if key.verify(&signature, &timestamp, &body).await.is_ok() {
                valid += 1;
            }
        }
    } else {
        for _ in 0..iterations {
            if composure::auth::validate_request(&public_key, &signature, &timestamp, &body)
                .await
                .is_ok()
            {
                valid += 1;
            }
        }
    }

    Response::ok(valid.to_string())
}
