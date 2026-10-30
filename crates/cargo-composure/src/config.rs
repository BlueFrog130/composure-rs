//! What `cargo composure dev` works on: the bot's package from `cargo metadata`, its settings from
//! `[package.metadata.composure.dev]` (flags win) and its secrets from `.env`.

use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
};

use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package, PackageId};
use composure::{
    credentials::{BOT_TOKEN_VAR, PUBLIC_KEY_VAR},
    types::Snowflake,
};
use serde::Deserialize;

/// The Worker's local port when `port` is not set (`wrangler dev`'s own default)
const WORKERS_PORT: u16 = 8787;
/// The server's local port when `port` is not set (the tokio adapter's own default)
const TOKIO_PORT: u16 = 8080;
const DEFAULT_SYNC_BIN: &str = "sync";

/// `[package.metadata.composure.dev]`
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// The Dev Application: the only application the tool writes to, and only with its own token
    pub application: Option<String>,
    /// The guild dev Sync sends Commands to
    pub guild: Option<String>,
    /// Which tunnel to open
    #[serde(default)]
    pub tunnel: TunnelSetting,
    /// The server bin (tokio)
    pub bin: Option<String>,
    /// The Sync bin
    pub sync_bin: Option<String>,
    /// The local port the bot serves on
    pub port: Option<u16>,
    /// `wrangler dev --env` (Workers)
    pub wrangler_env: Option<String>,
}

/// `tunnel = "quick"`, `tunnel = { url = ".." }` or `tunnel = false`
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawTunnel")]
pub enum TunnelSetting {
    /// A cloudflared quick tunnel on a new hostname each run
    #[default]
    Quick,
    /// A tunnel the developer already runs
    Url(String),
    /// No tunnel and no repointing
    Off,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawTunnel {
    Kind(String),
    On(bool),
    #[allow(dead_code)] // only `false` is valid, but `true` should get the error below
    Url {
        url: String,
    },
}

impl TryFrom<RawTunnel> for TunnelSetting {
    type Error = String;

    fn try_from(raw: RawTunnel) -> Result<Self, String> {
        match raw {
            RawTunnel::Kind(kind) if kind == "quick" => Ok(Self::Quick),
            RawTunnel::On(false) => Ok(Self::Off),
            RawTunnel::Url { url } if url.starts_with("https://") => {
                Ok(Self::Url(url.trim_end_matches('/').to_string()))
            }
            RawTunnel::Url { url } => Err(format!("tunnel url `{url}` must be https://")),
            RawTunnel::Kind(_) | RawTunnel::On(true) => {
                Err("tunnel must be \"quick\", { url = \"https://..\" } or false".to_string())
            }
        }
    }
}

/// How the bot runs locally
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// `wrangler dev`, from the config at this path
    Workers(PathBuf),
    /// The package's server bin
    Server(String),
}

/// A resolved project
#[derive(Debug)]
pub struct Project {
    pub package: String,
    /// The bot's package directory; every process runs here
    pub dir: PathBuf,
    pub target_dir: PathBuf,
    /// Every local package the bot is built from, its own first
    pub sources: Vec<PathBuf>,
    pub kind: Kind,
    pub application: Option<Snowflake>,
    pub guild: Option<Snowflake>,
    pub tunnel: TunnelSetting,
    pub sync_bin: Option<String>,
    pub port: u16,
    pub wrangler_env: Option<String>,
}

impl Project {
    /// Reads the package named `package`, or the one in the current directory
    pub fn load(
        package: Option<&str>,
        application: Option<String>,
        guild: Option<String>,
    ) -> Result<Self, String> {
        let metadata = MetadataCommand::new()
            .exec()
            .map_err(|e| format!("cargo metadata failed: {e}"))?;
        let root = package_of(&metadata, package)?;
        let settings = settings_of(root)?;
        let dir = root
            .manifest_path
            .parent()
            .expect("a manifest has a directory")
            .as_std_path()
            .to_path_buf();

        let wrangler = dir.join("wrangler.toml");
        let kind = match wrangler.exists() {
            true => Kind::Workers(wrangler),
            false => Kind::Server(server_bin(root, &settings)?),
        };
        let port = settings.port.unwrap_or(match kind {
            Kind::Workers(_) => WORKERS_PORT,
            Kind::Server(_) => TOKIO_PORT,
        });
        let sync_bin = settings.sync_bin.as_deref().unwrap_or(DEFAULT_SYNC_BIN);
        let has_sync_bin = root
            .targets
            .iter()
            .any(|t| t.is_bin() && t.name == sync_bin);

        Ok(Self {
            package: root.name.to_string(),
            target_dir: metadata.target_directory.as_std_path().to_path_buf(),
            sources: local_packages(&metadata, root),
            kind,
            application: snowflake("application", application.or(settings.application))?,
            guild: snowflake("guild", guild.or(settings.guild))?,
            tunnel: settings.tunnel,
            sync_bin: has_sync_bin.then(|| sync_bin.to_string()),
            port,
            wrangler_env: settings.wrangler_env,
            dir,
        })
    }
}

fn package_of<'a>(metadata: &'a Metadata, name: Option<&str>) -> Result<&'a Package, String> {
    match name {
        Some(name) => metadata
            .workspace_packages()
            .into_iter()
            .find(|p| p.name.as_str() == name)
            .ok_or_else(|| format!("no package `{name}` in this workspace")),
        None => metadata.root_package().ok_or_else(|| {
            "this directory has no package: run it in the bot's directory, or pass -p <package>"
                .to_string()
        }),
    }
}

fn settings_of(package: &Package) -> Result<Settings, String> {
    match package.metadata.pointer("/composure/dev") {
        Some(value) => Settings::deserialize(value)
            .map_err(|e| format!("[package.metadata.composure.dev]: {e}")),
        None => Ok(Settings::default()),
    }
}

/// `bin`, else `default-run`, else the only bin that isn't the Sync bin
fn server_bin(package: &Package, settings: &Settings) -> Result<String, String> {
    if let Some(bin) = settings.bin.as_ref().or(package.default_run.as_ref()) {
        return Ok(bin.clone());
    }
    let sync_bin = settings.sync_bin.as_deref().unwrap_or(DEFAULT_SYNC_BIN);
    let bins: Vec<_> = package
        .targets
        .iter()
        .filter(|t| t.is_bin() && t.name != sync_bin)
        .map(|t| t.name.as_str())
        .collect();
    match bins[..] {
        [bin] => Ok(bin.to_string()),
        [] => Err(format!("{} has no server bin", package.name)),
        _ => Err(format!(
            "{} has several bins ({}): set `bin` in [package.metadata.composure.dev]",
            package.name,
            bins.join(", ")
        )),
    }
}

/// An id, `None` when unset or empty (a template's prompt left blank)
fn snowflake(what: &str, id: Option<String>) -> Result<Option<Snowflake>, String> {
    id.filter(|id| !id.is_empty())
        .map(|id| id.parse().map_err(|_| format!("`{id}` is not a {what} id")))
        .transpose()
}

/// The directories of the local packages `root` is built from, through normal and build
/// dependencies: what a rebuild can depend on. Dev-dependencies never reach the bot.
fn local_packages(metadata: &Metadata, root: &Package) -> Vec<PathBuf> {
    let Some(resolve) = &metadata.resolve else {
        return Vec::new();
    };
    let mut seen: HashSet<&PackageId> = HashSet::new();
    let mut stack = vec![&root.id];
    let mut dirs = Vec::new();
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        let Some(package) = metadata.packages.iter().find(|p| &p.id == id) else {
            continue;
        };
        if package.source.is_some() {
            continue;
        }
        if let Some(dir) = package.manifest_path.parent() {
            dirs.push(dir.as_std_path().to_path_buf());
        }
        let Some(node) = resolve.nodes.iter().find(|n| &n.id == id) else {
            continue;
        };
        stack.extend(
            node.deps
                .iter()
                .filter(|dep| {
                    dep.dep_kinds
                        .iter()
                        .any(|k| k.kind != DependencyKind::Development)
                })
                .map(|dep| &dep.pkg),
        );
    }
    dirs
}

/// The secrets the Dev Loop itself reads
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Secrets {
    pub token: Option<String>,
    pub public_key: Option<String>,
}

impl Secrets {
    /// From `.env` in `dir`, under the process environment
    pub fn load(dir: &Path) -> Result<Self, String> {
        let path = dir.join(".env");
        let mut file = Vec::new();
        match dotenvy::from_path_iter(&path) {
            Ok(entries) => {
                for entry in entries {
                    file.push(entry.map_err(|e| format!("{}: {e}", path.display()))?);
                }
            }
            Err(e) if e.not_found() => {}
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
        Ok(Self::from_lookup(|name| {
            env::var(name).ok().or_else(|| {
                let (_, value) = file.iter().find(|(key, _)| key == name)?;
                Some(value.clone())
            })
        }))
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let set = |name| lookup(name).filter(|value| !value.is_empty());
        Self {
            token: set(BOT_TOKEN_VAR),
            public_key: set(PUBLIC_KEY_VAR),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn settings(value: serde_json::Value) -> Result<Settings, String> {
        Settings::deserialize(value).map_err(|e| e.to_string())
    }

    #[test]
    fn tunnel_takes_one_of_three_shapes() {
        let tunnel = |value| settings(json!({ "tunnel": value })).map(|s| s.tunnel);
        assert_eq!(settings(json!({})).unwrap().tunnel, TunnelSetting::Quick);
        assert_eq!(tunnel(json!("quick")), Ok(TunnelSetting::Quick));
        assert_eq!(tunnel(json!(false)), Ok(TunnelSetting::Off));
        assert_eq!(
            tunnel(json!({ "url": "https://bot.example/" })),
            Ok(TunnelSetting::Url("https://bot.example".into()))
        );

        assert!(tunnel(json!(true)).is_err());
        assert!(tunnel(json!("named")).is_err());
        assert!(tunnel(json!({ "url": "http://bot.example" })).is_err());
    }

    #[test]
    fn unknown_settings_are_errors() {
        assert!(settings(json!({ "guilds": "1" })).is_err());
    }

    #[test]
    fn blank_ids_are_unset() {
        assert_eq!(snowflake("guild", Some(String::new())), Ok(None));
        assert_eq!(snowflake("guild", None), Ok(None));
        assert!(snowflake("guild", Some("abc".into())).is_err());
    }

    #[test]
    fn empty_secrets_are_unset() {
        let secrets = Secrets::from_lookup(|name| match name {
            "DISCORD_TOKEN" => Some("t".to_string()),
            _ => Some(String::new()),
        });
        assert_eq!(secrets.token.as_deref(), Some("t"));
        assert_eq!(secrets.public_key, None);
    }
}
