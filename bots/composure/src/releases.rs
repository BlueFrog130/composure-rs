//! The `composure` crate's releases, from the GitHub Releases API (knope writes each version's
//! changelog section into its release). Every crate is released on its own (ADR 0016); only
//! `composure`'s are followed, and the shared `v<version>` releases up to 0.1.3. The cron trigger refreshes the copy in the [`Store`]; handlers read
//! only that copy, fetching once if it is missing.

use std::borrow::Cow;

use composure::{command::HandlerError, rest::Transport};
use serde::{Deserialize, Serialize};

use crate::{Config, Services, Store, http};

/// Where the releases are kept, newest first
pub const RELEASES_KEY: &str = "releases";

/// One published release
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Release {
    /// `0.1.2`, without the tag's `composure/v` or `v`
    pub version: String,
    /// `2026-10-20`
    pub date: String,
    /// The release's page on GitHub
    pub url: String,
    /// The release notes, as markdown
    pub notes: String,
}

/// What the GitHub API returns per release, as far as read
#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    published_at: Option<String>,
    draft: bool,
    prerelease: bool,
}

/// The version a `composure` release tag names: `composure/v0.1.4`, or `v0.1.3` from when every
/// crate shared one; `None` for the other crates' tags
fn composure_version(tag: &str) -> Option<&str> {
    tag.strip_prefix("composure/v")
        .or_else(|| tag.strip_prefix('v'))
}

/// Published (not draft, not prerelease) `composure` releases from GitHub, newest first
pub async fn fetch(
    transport: &impl Transport,
    config: &Config,
) -> Result<Vec<Release>, HandlerError> {
    let mut headers = vec![("x-github-api-version", Cow::Borrowed("2022-11-28"))];
    if let Some(token) = &config.github_token {
        headers.push(("authorization", Cow::Owned(format!("Bearer {token}"))));
    }
    let url = format!(
        "https://api.github.com/repos/{}/releases?per_page=100",
        config.repo
    );
    let releases: Vec<GitHubRelease> = http::get_json(transport, url, headers).await?;
    let mut releases: Vec<_> = releases
        .into_iter()
        .filter(|release| !release.draft && !release.prerelease)
        .filter_map(|release| {
            let version = composure_version(&release.tag_name)?.to_string();
            let published = release.published_at?;
            Some(Release {
                version,
                date: published.get(..10).unwrap_or(&published).to_string(),
                url: release.html_url,
                notes: release.body.unwrap_or_default(),
            })
        })
        .collect();
    // ISO dates sort as text; the API's order is by creation, not publication
    releases.sort_by(|a, b| {
        b.date
            .cmp(&a.date)
            .then_with(|| newer(&b.version, &a.version))
    });
    Ok(releases)
}

/// Orders `a` and `b` by their numeric `x.y.z` parts
fn newer(a: &str, b: &str) -> std::cmp::Ordering {
    let parts = |v: &str| -> Vec<u64> { v.split('.').filter_map(|p| p.parse().ok()).collect() };
    parts(a).cmp(&parts(b))
}

/// Fetches the releases and keeps them in `store`, writing only when they changed (KV writes
/// are the scarce ones, and the cron trigger refreshes every 15 minutes)
pub async fn refresh(
    transport: &impl Transport,
    store: &impl Store,
    config: &Config,
) -> Result<Vec<Release>, HandlerError> {
    let releases = fetch(transport, config).await?;
    let bytes = serde_json::to_vec(&releases).map_err(HandlerError::new)?;
    if store.get(RELEASES_KEY).await?.as_ref() != Some(&bytes) {
        store.put(RELEASES_KEY, bytes, None).await?;
    }
    Ok(releases)
}

/// The stored releases; fetched and stored when missing (before the cron trigger first ran)
pub async fn load(state: &impl Services) -> Result<Vec<Release>, HandlerError> {
    if let Some(bytes) = state.get(RELEASES_KEY).await?
        && let Ok(releases) = serde_json::from_slice(&bytes)
    {
        return Ok(releases);
    }
    refresh(&state.transport(), state, state.data()).await
}

/// The index of `version` (`0.1.2` or `v0.1.2`)
pub fn position(releases: &[Release], version: &str) -> Option<usize> {
    let version = version.trim().trim_start_matches('v');
    releases
        .iter()
        .position(|release| release.version == version)
}

/// The first `x.y.z` version in `text`, e.g. `composure v0.1.2` → `0.1.2`
pub fn find_version(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        let boundary =
            start == 0 || !(bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.');
        if boundary && bytes[start].is_ascii_digit() {
            let mut end = start;
            let mut dots = 0;
            while end < bytes.len() {
                match bytes[end] {
                    b'0'..=b'9' => end += 1,
                    b'.' if dots < 2 && bytes.get(end + 1).is_some_and(u8::is_ascii_digit) => {
                        dots += 1;
                        end += 1;
                    }
                    _ => break,
                }
            }
            if dots == 2 {
                return Some(&text[start..end]);
            }
            start = end.max(start + 1);
        } else {
            start += 1;
        }
    }
    None
}

/// Release notes made for Discord: headings one level down (the page has its own `##`),
/// `####` and below bold, and hard-wrapped paragraphs joined (Discord keeps every newline)
pub fn tidy(notes: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut fenced = false;
    let mut joinable = false;
    for line in notes.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            fenced = !fenced;
            lines.push(trimmed.to_string());
            joinable = false;
            continue;
        }
        if fenced {
            lines.push(line.trim_end().to_string());
            continue;
        }
        if trimmed.is_empty() {
            if lines.last().is_some_and(|last| !last.is_empty()) {
                lines.push(String::new());
            }
            joinable = false;
            continue;
        }
        if let Some(heading) = heading(trimmed) {
            lines.push(heading);
            joinable = false;
            continue;
        }
        let starts_block = trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with('>')
            || trimmed.starts_with('|')
            || trimmed
                .split_once(". ")
                .is_some_and(|(n, _)| n.parse::<u32>().is_ok());
        match lines.last_mut() {
            Some(last) if joinable && !starts_block => {
                last.push(' ');
                last.push_str(trimmed);
            }
            _ => lines.push(line.trim_end().to_string()),
        }
        joinable = !trimmed.starts_with('|');
    }
    lines.join("\n").trim().to_string()
}

fn heading(line: &str) -> Option<String> {
    let level = line.bytes().take_while(|&b| b == b'#').count();
    let text = line[level..].strip_prefix(' ')?.trim();
    Some(match level {
        1 | 2 => format!("### {text}"),
        _ => format!("**{text}**"),
    })
}

/// At most `max` characters of `text`, cut at a paragraph (or line) break, code fences closed;
/// whether anything was cut
pub fn excerpt(text: &str, max: usize) -> (String, bool) {
    if text.chars().count() <= max {
        return (text.to_string(), false);
    }
    let end = text.char_indices().nth(max).map_or(text.len(), |(i, _)| i);
    let head = &text[..end];
    let cut = head
        .rfind("\n\n")
        .or_else(|| head.rfind('\n'))
        .unwrap_or(end);
    let mut excerpt = text[..cut].trim_end().to_string();
    if excerpt.matches("```").count() % 2 == 1 {
        excerpt.push_str("\n```");
    }
    (excerpt, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_versions() {
        assert_eq!(find_version("composure v0.1.2 is out"), Some("0.1.2"));
        assert_eq!(find_version("10.20.300"), Some("10.20.300"));
        assert_eq!(find_version("1.2 and 3.4.5."), Some("3.4.5"));
        assert_eq!(find_version("1.2.3.4"), Some("1.2.3"));
        assert_eq!(find_version("no version 1.2"), None);
    }

    #[test]
    fn tidies_notes() {
        let notes = "## Features\n\n### Commands\n\n`#[command]` turns\nfunctions into commands.\n\n- one\n- two\n  continued\n\n```rust\nlet x =\n    1;\n```";
        assert_eq!(
            tidy(notes),
            "### Features\n\n**Commands**\n\n`#[command]` turns functions into commands.\n\n- one\n- two continued\n\n```rust\nlet x =\n    1;\n```"
        );
    }

    #[test]
    fn excerpts_at_breaks() {
        assert_eq!(excerpt("short", 10), ("short".into(), false));
        assert_eq!(excerpt("one\n\ntwo two two", 10), ("one".into(), true));
        assert_eq!(
            excerpt("```\ncode\n\nmore code", 12),
            ("```\ncode\n```".into(), true)
        );
    }

    #[test]
    fn reads_composure_tags() {
        assert_eq!(composure_version("composure/v0.1.4"), Some("0.1.4"));
        assert_eq!(composure_version("v0.1.3"), Some("0.1.3"));
        assert_eq!(composure_version("composure_macros/v0.1.4"), None);
        assert_eq!(composure_version("composure_adapter_tokio/v0.2.0"), None);
    }

    #[test]
    fn orders_versions() {
        assert!(newer("0.10.0", "0.9.0").is_gt());
        assert!(newer("0.1.0", "0.1.1").is_lt());
    }
}
