//! `/crate [name]`: a crate's crates.io stats, fetched as Background Work after a Deferred
//! Response and cached for 10 minutes.

use std::time::Duration;

use composure::{
    Choice, Ctx, command,
    command::{Deferred, HandlerError, View},
    rsx,
};
use serde::{Deserialize, Serialize};

use crate::{Services, http, ui};

const CACHE_TTL: Duration = Duration::from_secs(10 * 60);

/// The published crates
#[derive(Choice, Debug, Clone, Copy)]
pub enum CrateName {
    #[choice(name = "composure", value = "composure")]
    Composure,
    #[choice(name = "composure_macros", value = "composure_macros")]
    Macros,
    #[choice(name = "composure_commands", value = "composure_commands")]
    Commands,
    #[choice(name = "composure_sync", value = "composure_sync")]
    Sync,
    #[choice(
        name = "composure_adapter_cloudflare",
        value = "composure_adapter_cloudflare"
    )]
    Cloudflare,
    #[choice(name = "composure_adapter_tokio", value = "composure_adapter_tokio")]
    Tokio,
}

/// The stats shown, as cached
#[derive(Debug, Serialize, Deserialize)]
pub struct Stats {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub downloads: u64,
    pub recent_downloads: Option<u64>,
    pub updated: String,
}

#[derive(Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    krate: CrateData,
}

#[derive(Deserialize)]
struct CrateData {
    name: String,
    description: Option<String>,
    max_stable_version: Option<String>,
    max_version: String,
    downloads: u64,
    recent_downloads: Option<u64>,
    updated_at: String,
}

/// Stats for `name`, from the cache or crates.io
pub async fn stats(state: &impl Services, name: &str) -> Result<Stats, HandlerError> {
    let key = format!("crate:{name}");
    if let Some(bytes) = state.get(&key).await?
        && let Ok(stats) = serde_json::from_slice(&bytes)
    {
        return Ok(stats);
    }
    let response: CrateResponse = http::get_json(
        &state.transport(),
        format!("https://crates.io/api/v1/crates/{name}"),
        Vec::new(),
    )
    .await?;
    let krate = response.krate;
    let stats = Stats {
        version: krate.max_stable_version.unwrap_or(krate.max_version),
        name: krate.name,
        description: krate.description,
        downloads: krate.downloads,
        recent_downloads: krate.recent_downloads,
        updated: krate
            .updated_at
            .get(..10)
            .unwrap_or(&krate.updated_at)
            .to_string(),
    };
    let bytes = serde_json::to_vec(&stats).map_err(HandlerError::new)?;
    state.put(&key, bytes, Some(CACHE_TTL)).await?;
    Ok(stats)
}

/// `1234567` → `1,234,567`
fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The stats as a card
pub fn card(stats: &Stats) -> View {
    let name = stats.name.as_str();
    rsx! {
        <ui::Primary>
            <Text>"## " {name} " " {stats.version.as_str()}</Text>
            {stats.description.as_deref().map(|description| rsx! { <Text>{description}</Text> })}
            <Separator/>
            <Text>
                "**" {thousands(stats.downloads)} "** downloads"
                {stats.recent_downloads.map(|recent| format!(" · **{}** in the last 90 days", thousands(recent)))}
                "\n-# Updated " {stats.updated.as_str()}
            </Text>
            <Row>
                <Button url={format!("https://crates.io/crates/{name}")}>"crates.io"</Button>
                <Button url={format!("https://docs.rs/{name}")}>"docs.rs"</Button>
                <Button url={format!("https://lib.rs/crates/{name}")}>"lib.rs"</Button>
            </Row>
        </ui::Primary>
    }
}

/// Show a Composure crate's version and downloads
#[command(
    name = "crate",
    contexts(Guild, BotDm, PrivateChannel),
    integration_types(GuildInstall, UserInstall)
)]
pub fn crate_info(
    ctx: Ctx<impl Services>,
    #[option("Which crate (default: composure)")] name: Option<CrateName>,
) -> Deferred {
    let name = name.unwrap_or(CrateName::Composure).value();
    // crates.io may take a while; the Deferred Response buys up to 15 minutes
    ctx.defer(async move |ctx| {
        let stats = stats(ctx.state(), name).await?;
        Ok::<_, HandlerError>(card(&stats))
    })
}

#[cfg(test)]
mod tests {
    use super::thousands;

    #[test]
    fn groups_digits() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(1234567), "1,234,567");
    }
}
