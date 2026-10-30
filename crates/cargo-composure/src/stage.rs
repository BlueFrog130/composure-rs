//! The Dev Loop's stages: one `[composure]` line each, and failures that name their stage.

use std::{
    fmt,
    sync::Mutex,
    time::{Duration, Instant},
};

/// One line of the Dev Loop's own output
macro_rules! info {
    ($($arg:tt)*) => { eprintln!("[composure] {}", format_args!($($arg)*)) };
}

/// A line about something that needs the developer
macro_rules! warning {
    ($($arg:tt)*) => { eprintln!("[composure] warning: {}", format_args!($($arg)*)) };
}

pub(crate) use {info, warning};

/// A stage that failed: logged as one line, the Dev Loop keeps running
#[derive(Debug, PartialEq, Eq)]
pub struct StageError {
    /// `tunnel`, `dns`, `probe`, `repoint`
    pub stage: &'static str,
    /// What went wrong, with the last thing seen
    pub message: String,
}

impl StageError {
    pub fn new(stage: &'static str, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
        }
    }
}

impl fmt::Display for StageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} failed: {}", self.stage, self.message)
    }
}

/// The last thing a stage saw, quoted when it runs out of time
#[derive(Debug, Default)]
pub struct Seen(Mutex<String>);

impl Seen {
    pub fn observe(&self, what: impl Into<String>) {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = what.into();
    }

    fn last(&self) -> String {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// Runs `work` for at most `limit`, failing `stage` with what it was waiting for
pub async fn within<T>(
    stage: &'static str,
    limit: Duration,
    wanted: &str,
    work: impl AsyncFnOnce(&Seen) -> Result<T, StageError>,
) -> Result<T, StageError> {
    let seen = Seen::default();
    match tokio::time::timeout(limit, work(&seen)).await {
        Ok(result) => result,
        Err(_) => {
            let last = seen.last();
            let last = match last.is_empty() {
                true => "nothing".to_string(),
                false => last,
            };
            let seconds = limit.as_secs();
            Err(StageError::new(
                stage,
                format!("no {wanted} within {seconds}s; last saw: {last}"),
            ))
        }
    }
}

/// `6.2s`
pub fn elapsed(since: Instant) -> String {
    format!("{:.1}s", since.elapsed().as_secs_f32())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn running_out_of_time_quotes_the_last_thing_seen() {
        let result: Result<(), _> = within("probe", Duration::from_secs(30), "401", async |seen| {
            seen.observe("HTTP 502");
            std::future::pending().await
        })
        .await;
        assert_eq!(
            result.unwrap_err().to_string(),
            "probe failed: no 401 within 30s; last saw: HTTP 502"
        );
    }
}
