//! The Dev Loop against real cloudflared and Discord: Discord accepts the tunnel only after its
//! signed PING reaches the bot. Run before changing the Tunnel or Repoint; it repoints the
//! development application and overwrites its Commands in the test guild, from
//! `examples/tokio/.env` (`DISCORD_PUBLIC_KEY`, `DISCORD_TOKEN`) and the example's
//! `[package.metadata.composure.dev]`:
//!
//! `cargo test -p cargo-composure --test live -- --ignored --nocapture`

use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

const LIMIT: Duration = Duration::from_secs(300);

/// Stops the tool and what it started, whatever the test did
struct Tool(Child);

impl Drop for Tool {
    fn drop(&mut self) {
        let id = self.0.id().to_string();
        #[cfg(windows)]
        let _ = Command::new("taskkill")
            .args(["/F", "/T", "/PID", &id])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        #[cfg(not(windows))]
        let _ = Command::new("kill").args(["-s", "INT", &id]).status();
        let _ = self.0.wait();
    }
}

#[test]
#[ignore = "needs cloudflared, and writes to the development application"]
fn discord_accepts_the_tunnel_and_commands_reach_the_guild() {
    let mut tool = Tool(
        Command::new(env!("CARGO_BIN_EXE_cargo-composure"))
            .args(["composure", "dev", "-p", "composure_example_tokio"])
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap(),
    );

    let (lines, received) = mpsc::channel();
    let stderr = tool.0.stderr.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            eprintln!("{line}");
            let _ = lines.send(line);
        }
    });

    let started = Instant::now();
    let (mut repointed, mut synced) = (false, false);
    while !(repointed && synced) {
        let left = LIMIT.saturating_sub(started.elapsed());
        let line = received
            .recv_timeout(left)
            .expect("the Dev Loop finished its stages in time");
        assert!(
            !line.contains("failed:") && !line.contains("is not DISCORD_TOKEN's application"),
            "{line}"
        );
        repointed |= line.contains("Dev Application") && line.contains(" -> ");
        synced |= line.contains("dev Sync to guild");
    }
}
