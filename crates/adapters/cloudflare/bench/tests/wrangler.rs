//! Benchmarks `composure::auth::validate_request` on the real Workers runtime (`wrangler dev`,
//! workerd): ed25519-dalek compiled to wasm vs the `web-crypto` feature.
//!
//! Workers freeze timers while code runs, so the worker can't time itself. Each request runs
//! validation N times; per-verify time is `(median(N) - median(1)) / (N - 1)`, which cancels the
//! HTTP, wrangler proxy, and body-read overhead.
//!
//! Needs Node (`npx wrangler`) and `worker-build` (`cargo install worker-build@0.8.7`).
//! Run with `cargo test -p composure_bench_cloudflare --test wrangler --release -- --ignored --nocapture`.
//! `WRANGLER_PORT` (default 8798), `BENCH_ITERATIONS` (default 200), and `BENCH_SAMPLES`
//! (default 15) override the defaults.

mod common;

use std::{
    env, fs,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

use common::Wrangler;
use ed25519_dalek::{Signer, SigningKey};
use reqwest::blocking::Client;

const SEED: [u8; 32] = [7; 32];
const TIMESTAMP: &str = "1760000000";
/// Typical interaction, one with lots of resolved data, and an outlier
const BODY_SIZES: [usize; 3] = [1024, 16 * 1024, 128 * 1024];

struct Backend {
    name: &'static str,
    features: &'static [&'static str],
}

const BACKENDS: [Backend; 2] = [
    Backend {
        name: "dalek",
        features: &[],
    },
    Backend {
        name: "web-crypto",
        features: &["--features", "web-crypto"],
    },
];

struct Signed {
    public_key: String,
    signature: String,
    body: Vec<u8>,
}

/// An interaction-shaped JSON body of exactly `size` bytes, signed
fn signed_body(size: usize) -> Signed {
    let prefix = br#"{"type":2,"token":""#;
    let suffix = br#""}"#;
    let mut body = prefix.to_vec();
    body.resize(size - suffix.len(), b'a');
    body.extend_from_slice(suffix);

    let key = SigningKey::from_bytes(&SEED);
    let message = [TIMESTAMP.as_bytes(), &body].concat();
    Signed {
        public_key: hex::encode(key.verifying_key().as_bytes()),
        signature: hex::encode(key.sign(&message).to_bytes()),
        body,
    }
}

fn env_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// One request running `iterations` validations; asserts every one passed
fn time_request(
    client: &Client,
    url: &str,
    signed: &Signed,
    cached: bool,
    iterations: u32,
) -> Duration {
    let started = Instant::now();
    let response = client
        .post(url)
        .header("X-Cached-Key", if cached { "1" } else { "0" })
        .header("X-Public-Key", &signed.public_key)
        .header("X-Signature-Ed25519", &signed.signature)
        .header("X-Signature-Timestamp", TIMESTAMP)
        .header("X-Iterations", iterations.to_string())
        .body(signed.body.clone())
        .send()
        .unwrap();
    let valid = response.text().unwrap();
    let elapsed = started.elapsed();

    assert_eq!(valid, iterations.to_string(), "not every validation passed");
    elapsed
}

fn median(
    client: &Client,
    url: &str,
    signed: &Signed,
    cached: bool,
    iterations: u32,
    samples: usize,
) -> Duration {
    let mut times: Vec<_> = (0..samples)
        .map(|_| time_request(client, url, signed, cached, iterations))
        .collect();
    times.sort();
    times[samples / 2]
}

fn build(dir: &Path, backend: &Backend) -> String {
    let out_dir = format!("build-{}", backend.name);
    let status = Command::new("worker-build")
        .args(["--release", "-d", &out_dir, "."])
        .args(backend.features)
        .current_dir(dir)
        .status()
        .expect("failed to run worker-build");
    assert!(status.success(), "worker-build failed for {}", backend.name);
    out_dir
}

struct Row {
    backend: &'static str,
    /// Key parsed once (as the adapter does) or per verify (`validate_request`)
    cached: bool,
    wasm_bytes: u64,
    body_bytes: usize,
    per_verify: Duration,
}

#[test]
#[ignore = "benchmark; starts wrangler dev, needs Node and worker-build"]
fn validate_request_speed() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let port: u16 = env_or("WRANGLER_PORT", 8798);
    let iterations: u32 = env_or("BENCH_ITERATIONS", 200);
    let samples: usize = env_or("BENCH_SAMPLES", 15);
    assert!(iterations > 1 && samples > 0);

    let url = format!("http://127.0.0.1:{port}/");
    let client = Client::new();
    let mut rows = Vec::new();

    for backend in &BACKENDS {
        let out_dir = build(dir, backend);
        let wasm_bytes = fs::metadata(dir.join(&out_dir).join("index_bg.wasm"))
            .map(|m| m.len())
            .unwrap_or(0);

        let script = format!("{out_dir}/worker/shim.mjs");
        let mut wrangler = Wrangler::start(dir, port, &[&script]);
        wrangler.wait_ready(port, Duration::from_secs(120));

        for cached in [false, true] {
            for size in BODY_SIZES {
                let signed = signed_body(size);
                // Warm up: wasm tier-up and Web Crypto init
                for _ in 0..5 {
                    time_request(&client, &url, &signed, cached, iterations);
                }

                let one = median(&client, &url, &signed, cached, 1, samples);
                let many = median(&client, &url, &signed, cached, iterations, samples);
                rows.push(Row {
                    backend: backend.name,
                    cached,
                    wasm_bytes,
                    body_bytes: size,
                    per_verify: many.saturating_sub(one) / (iterations - 1),
                });
            }
        }

        drop(wrangler);
    }

    println!("\nvalidate_request on wrangler dev ({iterations} iterations, median of {samples})\n");
    println!(
        "| {:<10} | {:<6} | {:>10} | {:>8} | {:>10} | {:>10} |",
        "backend", "key", "wasm", "body", "µs/verify", "verify/s"
    );
    println!(
        "|{:-<12}|{:-<8}|{:->12}|{:->10}|{:->12}|{:->12}|",
        "", "", "", "", "", ""
    );
    for row in &rows {
        let micros = row.per_verify.as_secs_f64() * 1e6;
        println!(
            "| {:<10} | {:<6} | {:>8}KB | {:>6}KB | {:>10.1} | {:>10.0} |",
            row.backend,
            if row.cached { "cached" } else { "hex" },
            row.wasm_bytes / 1024,
            row.body_bytes / 1024,
            micros,
            1e6 / micros
        );
    }
}
