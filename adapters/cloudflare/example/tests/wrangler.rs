//! Every fixture over HTTP against the regression bot running in `wrangler dev` (workerd), so
//! the real Workers runtime, request/header handling, secrets, and response encoding are covered.
//!
//! Needs Node (`npx wrangler`) and `worker-build` (`cargo install worker-build@0.8.7`).
//! Writes `.dev.vars` with the fixtures' public key, starts wrangler, and stops it afterwards.
//! Run with `cargo test -p composure_example_cloudflare --test wrangler -- --ignored --nocapture`.
//! `WRANGLER_PORT` overrides the port (default 8799).

mod common;

use std::{
    env, fs,
    net::TcpStream,
    path::Path,
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;

/// Stops wrangler and everything it spawned when dropped
struct Wrangler(Child);

impl Wrangler {
    fn start(dir: &Path, port: u16) -> Self {
        let args = [
            "wrangler",
            "dev",
            "--ip",
            "127.0.0.1",
            "--port",
            &port.to_string(),
        ];
        let mut command = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.arg("/C").arg("npx").args(args);
            c
        } else {
            let mut c = Command::new("npx");
            c.args(args);
            c
        };
        #[cfg(unix)]
        std::os::unix::process::CommandExt::process_group(&mut command, 0);

        Wrangler(
            command
                .current_dir(dir)
                .spawn()
                .expect("failed to run npx wrangler"),
        )
    }

    fn wait_ready(&mut self, port: u16, timeout: Duration) {
        let started = Instant::now();
        while started.elapsed() < timeout {
            if let Some(status) = self.0.try_wait().unwrap() {
                panic!("wrangler exited early: {status}");
            }
            if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                return;
            }
            thread::sleep(Duration::from_millis(500));
        }
        panic!("wrangler not listening on {port} after {timeout:?}");
    }
}

impl Drop for Wrangler {
    fn drop(&mut self) {
        let pid = self.0.id().to_string();
        if cfg!(windows) {
            let _ = Command::new("taskkill")
                .args(["/PID", &pid, "/T", "/F"])
                .output();
        } else {
            let _ = Command::new("kill")
                .args(["-TERM", &format!("-{pid}")])
                .output();
        }
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "starts wrangler dev; needs Node and worker-build"]
fn signed_interactions_over_http() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let port: u16 = env::var("WRANGLER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8799);

    fs::write(
        dir.join(".dev.vars"),
        format!("DISCORD_PUBLIC_KEY={}\n", common::public_key()),
    )
    .unwrap();

    let mut wrangler = Wrangler::start(dir, port);
    // First run compiles the worker in release mode
    wrangler.wait_ready(port, Duration::from_secs(600));

    let url = format!("http://127.0.0.1:{port}/");
    let client = reqwest::blocking::Client::new();
    let mut failures = Vec::new();

    for case in common::cases() {
        let (signature, timestamp, body) = case.request();
        let mut request = client
            .post(&url)
            .header("Content-Type", "application/json")
            .body(body);
        if let Some(signature) = signature {
            request = request.header("X-Signature-Ed25519", signature);
        }
        if let Some(timestamp) = timestamp {
            request = request.header("X-Signature-Timestamp", timestamp);
        }

        let response = request.send().unwrap();
        let status = response.status().as_u16();
        let text = response.text().unwrap();

        if status != case.status {
            failures.push(format!(
                "{}: expected {}, got {status}: {text}",
                case.name, case.status
            ));
            continue;
        }
        if let Some(expected) = &case.response {
            match serde_json::from_str::<Value>(&text) {
                Ok(actual) if &actual == expected => {}
                _ => failures.push(format!(
                    "{}:\n  expected {expected}\n  got      {text}",
                    case.name
                )),
            }
        }
    }

    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
