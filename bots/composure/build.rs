//! Generates `$OUT_DIR/build_info.rs`: the workspace's Composure version, the git commit, and the
//! `/docs` index of the guide (every chapter in `book/src/SUMMARY.md` and its headings).

use std::{env, fmt::Write as _, fs, path::Path, process::Command};

const BOOK_URL: &str = "https://bluefrog130.github.io/composure-rs/";

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let book = root.join("book/src");
    println!("cargo::rerun-if-changed={}", book.display());
    println!(
        "cargo::rerun-if-changed={}",
        root.join("Cargo.toml").display()
    );
    println!(
        "cargo::rerun-if-changed={}",
        root.join(".git/HEAD").display()
    );
    println!("cargo::rerun-if-env-changed=GITHUB_SHA");

    let mut out = String::new();
    let version = workspace_version(&fs::read_to_string(root.join("Cargo.toml")).unwrap());
    writeln!(out, "pub const COMPOSURE_VERSION: &str = {version:?};").unwrap();
    writeln!(out, "pub const GIT_SHA: &str = {:?};", git_sha()).unwrap();

    out.push_str("pub static GUIDE: &[crate::docs::Entry] = &[\n");
    let summary = fs::read_to_string(book.join("SUMMARY.md")).unwrap();
    for (title, file) in chapters(&summary) {
        let text = fs::read_to_string(book.join(&file)).unwrap();
        let page = file.trim_end_matches(".md").to_string() + ".html";
        for entry in entries(&title, &page, &text) {
            writeln!(
                out,
                "    crate::docs::Entry {{ title: {:?}, url: {:?}, summary: {:?} }},",
                entry.0, entry.1, entry.2
            )
            .unwrap();
        }
    }
    out.push_str("];\n");

    let path = Path::new(&env::var("OUT_DIR").unwrap()).join("build_info.rs");
    fs::write(path, out).unwrap();
}

/// `version` under `[workspace.package]`
fn workspace_version(manifest: &str) -> String {
    let mut in_package = false;
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
        } else if in_package && let Some(value) = line.strip_prefix("version = ") {
            return value.trim_matches('"').to_string();
        }
    }
    panic!("no version in [workspace.package]");
}

/// The short commit: CI's `GITHUB_SHA`, else git, else "unknown"
fn git_sha() -> String {
    if let Ok(sha) = env::var("GITHUB_SHA") {
        return sha.chars().take(7).collect();
    }
    Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|sha| sha.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

/// (title, file) of every `[Title](file.md)` in SUMMARY.md
fn chapters(summary: &str) -> Vec<(String, String)> {
    summary
        .lines()
        .filter_map(|line| {
            let (title, rest) = line.split_once('[')?.1.split_once("](")?;
            let file = rest.split_once(')')?.0;
            Some((title.to_string(), file.to_string()))
        })
        .collect()
}

/// (title, url, summary) for the chapter itself and each `##`-`####` heading in it, outside code
fn entries(chapter: &str, page: &str, text: &str) -> Vec<(String, String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut entries = vec![(
        chapter.to_string(),
        format!("{BOOK_URL}{page}"),
        paragraph_after(&lines, 0),
    )];
    let mut fenced = false;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("```") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let Some(heading) = ["## ", "### ", "#### "]
            .iter()
            .find_map(|marker| line.strip_prefix(marker))
        else {
            continue;
        };
        let title = format!("{chapter} › {}", plain(heading));
        entries.push((
            title.chars().take(100).collect(),
            format!("{BOOK_URL}{page}#{}", slug(heading)),
            paragraph_after(&lines, i + 1),
        ));
    }
    entries
}

/// The first prose paragraph from line `start` on, links reduced to their text, at most ~300
/// characters
fn paragraph_after(lines: &[&str], start: usize) -> String {
    let mut paragraph = String::new();
    for line in lines.iter().skip(start).map(|line| line.trim()) {
        if line.starts_with('#') && paragraph.is_empty() {
            continue;
        }
        if line.is_empty() {
            if paragraph.is_empty() {
                continue;
            }
            break;
        }
        if line.starts_with("```") || line.starts_with('|') || line.starts_with('#') {
            break;
        }
        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(line);
    }
    let paragraph = strip_links(&paragraph);
    if paragraph.chars().count() <= 300 {
        return paragraph;
    }
    let cut: String = paragraph.chars().take(300).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{cut}…")
}

/// `[text](url)` → `text`
fn strip_links(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find("](").map(|i| open + i) else {
            break;
        };
        let Some(end) = rest[close..].find(')').map(|i| close + i) else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push_str(&rest[open + 1..close]);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// A heading without markdown: links reduced to text, backticks dropped
fn plain(heading: &str) -> String {
    strip_links(heading).replace('`', "")
}

/// mdBook's anchor for a heading: lowercase, spaces to `-`, only alphanumerics, `-` and `_` kept
fn slug(heading: &str) -> String {
    strip_links(heading)
        .trim()
        .chars()
        .filter_map(|c| match c {
            ' ' => Some('-'),
            c if c.is_alphanumeric() || c == '-' || c == '_' => Some(c.to_ascii_lowercase()),
            _ => None,
        })
        .collect()
}
