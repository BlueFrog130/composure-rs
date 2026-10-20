//! Runs `npx wrangler dev` for the `wrangler dev` tests.

use std::{
    net::TcpStream,
    path::Path,
    process::{Child, Command},
    thread,
    time::{Duration, Instant},
};

/// Stops wrangler and everything it spawned when dropped
pub struct Wrangler(Child);

impl Wrangler {
    /// Starts `npx wrangler dev` in `dir`; `extra` is appended (e.g. a script path)
    pub fn start(dir: &Path, port: u16, extra: &[&str]) -> Self {
        let port = port.to_string();
        let mut args = vec!["wrangler", "dev", "--ip", "127.0.0.1", "--port", &port];
        args.extend_from_slice(extra);
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

    pub fn wait_ready(&mut self, port: u16, timeout: Duration) {
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
