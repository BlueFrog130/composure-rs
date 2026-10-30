//! Tunnels (ADR 0018): a public HTTPS URL forwarding to the bot's port, connected and ready for
//! Repoint. A quick tunnel's hostname is published 1 to 3 s after cloudflared registers a
//! connection, and a recursive lookup before that caches "no such host" for 60 s, at Discord too.
//! So it is ready once the zone's authoritative nameservers answer, and it carries that IP as
//! `address` for the probe.

use std::{env, io::ErrorKind, net::IpAddr, process::Stdio, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::Command,
    sync::mpsc,
    time::sleep,
};

use crate::{
    config::TunnelSetting,
    dns::{Dns, UdpDns},
    stage::{StageError, warning, within},
};

const CONNECT_LIMIT: Duration = Duration::from_secs(30);
const DNS_LIMIT: Duration = Duration::from_secs(60);
const DNS_INTERVAL: Duration = Duration::from_secs(1);
const CLOUDFLARED_PATH_VAR: &str = "CLOUDFLARED_PATH";
const INSTALL: &str =
    "https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/";

/// A tunnel that is connected
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tunnel {
    /// `https://..`, no trailing slash
    pub url: String,
    /// An IP the URL's host is known to resolve to, for probing before it resolves everywhere
    pub address: Option<IpAddr>,
}

/// Opens the configured tunnel; `None` for `tunnel = false`. A quick tunnel lives until the Dev
/// Loop's runtime shuts down, which kills cloudflared.
pub async fn open(setting: &TunnelSetting, port: u16) -> Result<Option<Tunnel>, StageError> {
    match setting {
        TunnelSetting::Off => Ok(None),
        TunnelSetting::Url(url) => Ok(Some(Tunnel {
            url: url.clone(),
            address: None,
        })),
        TunnelSetting::Quick => quick(port).await.map(Some),
    }
}

async fn quick(port: u16) -> Result<Tunnel, StageError> {
    let binary = env::var(CLOUDFLARED_PATH_VAR).unwrap_or_else(|_| "cloudflared".to_string());
    let mut child = Command::new(binary)
        .args(["tunnel", "--no-autoupdate", "--url"])
        .arg(format!("http://127.0.0.1:{port}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => StageError::new(
                "tunnel",
                format!("cloudflared is not installed: install it ({INSTALL}), or set {CLOUDFLARED_PATH_VAR}"),
            ),
            _ => StageError::new("tunnel", format!("starting cloudflared: {e}")),
        })?;

    // Both pipes are read for as long as cloudflared runs, or it blocks on a full one
    let (lines, mut received) = mpsc::unbounded_channel();
    tokio::spawn(forward(child.stdout.take(), lines.clone()));
    tokio::spawn(forward(child.stderr.take(), lines));

    let url = tokio::select! {
        url = registered(&mut received) => url?,
        status = child.wait() => {
            let code = exit_code(status);
            return Err(StageError::new(
                "tunnel",
                format!("cloudflared exited ({code}) before connecting"),
            ));
        }
    };
    let address = published(&UdpDns, &url, DNS_INTERVAL).await?;

    tokio::spawn(async move {
        // Ctrl-C reaches cloudflared too: that exit is expected
        tokio::select! {
            biased;
            _ = tokio::signal::ctrl_c() => {}
            status = child.wait() => {
                let code = exit_code(status);
                warning!("cloudflared exited ({code}); the quick tunnel is down");
            }
        }
    });
    Ok(Tunnel {
        url,
        address: Some(address),
    })
}

fn exit_code(status: std::io::Result<std::process::ExitStatus>) -> String {
    status.map_or_else(|e| e.to_string(), |s| s.to_string())
}

async fn forward(pipe: Option<impl AsyncRead + Unpin>, lines: mpsc::UnboundedSender<String>) {
    let Some(pipe) = pipe else { return };
    let mut reader = BufReader::new(pipe).lines();
    while let Ok(Some(line)) = reader.next_line().await {
        // Nobody listens once it has connected: this only keeps the pipe drained
        let _ = lines.send(line);
    }
}

/// The tunnel's URL, at cloudflared's first registered connection
async fn registered(lines: &mut mpsc::UnboundedReceiver<String>) -> Result<String, StageError> {
    within(
        "tunnel",
        CONNECT_LIMIT,
        "registered connection from cloudflared",
        async |seen| {
            let mut url = None;
            while let Some(line) = lines.recv().await {
                if !line.trim().is_empty() {
                    seen.observe(line.trim());
                }
                url = url.or_else(|| tunnel_url(&line));
                if let Some(url) = &url
                    && line.contains("Registered tunnel connection")
                {
                    return Ok(url.clone());
                }
            }
            Err(StageError::new("tunnel", "cloudflared closed its output"))
        },
    )
    .await
}

/// The `https://<words>.trycloudflare.com` in a line of cloudflared's output
fn tunnel_url(line: &str) -> Option<String> {
    const SUFFIX: &str = ".trycloudflare.com";
    let start = line.find("https://")?;
    let url = line[start..]
        .split(|c: char| !(c.is_ascii_alphanumeric() || "-.:/".contains(c)))
        .next()?
        .trim_end_matches('/');
    let name = url.strip_prefix("https://")?.strip_suffix(SUFFIX)?;
    (!name.is_empty() && !name.contains('.') && name != "api").then(|| url.to_string())
}

/// Waits for the zone's authoritative nameservers to publish the tunnel's hostname
async fn published(dns: &impl Dns, url: &str, interval: Duration) -> Result<IpAddr, StageError> {
    let host = url.trim_start_matches("https://");
    let zone = host.split_once('.').map_or(host, |(_, zone)| zone);
    let wanted = format!("A record for {host} from {zone}'s nameservers");
    within("dns", DNS_LIMIT, &wanted, async |seen| {
        let mut servers = None;
        loop {
            if servers.is_none() {
                match dns.nameservers(zone).await {
                    Ok(found) => servers = Some(found),
                    Err(error) => seen.observe(error),
                }
            }
            if let Some(servers) = &servers {
                match dns.a_record(servers, host).await {
                    Ok(Some(ip)) => return Ok(IpAddr::V4(ip)),
                    Ok(None) => seen.observe("no A record"),
                    Err(error) => seen.observe(error),
                }
            }
            sleep(interval).await;
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        net::Ipv4Addr,
        sync::{Arc, Mutex},
    };

    use super::*;

    #[derive(Default)]
    struct ScriptedDns {
        /// One answer per A query, then "no A record"
        answers: Mutex<VecDeque<Result<Option<Ipv4Addr>, String>>>,
        /// Every name asked for
        asked: Arc<Mutex<Vec<String>>>,
    }

    impl Dns for ScriptedDns {
        async fn nameservers(&self, zone: &str) -> Result<Vec<IpAddr>, String> {
            self.asked.lock().unwrap().push(format!("NS {zone}"));
            Ok(vec![IpAddr::V4(Ipv4Addr::LOCALHOST)])
        }

        async fn a_record(&self, _: &[IpAddr], host: &str) -> Result<Option<Ipv4Addr>, String> {
            self.asked.lock().unwrap().push(format!("A {host}"));
            self.answers.lock().unwrap().pop_front().unwrap_or(Ok(None))
        }
    }

    const URL: &str = "https://some-words-here.trycloudflare.com";

    #[test]
    fn finds_the_url_in_cloudflareds_box() {
        assert_eq!(
            tunnel_url(&format!("INF |  {URL}                 |")).as_deref(),
            Some(URL)
        );
        assert_eq!(
            tunnel_url("INF Requesting new quick Tunnel on trycloudflare.com..."),
            None
        );
        assert_eq!(
            tunnel_url("failed to request https://api.trycloudflare.com/tunnel"),
            None
        );
        assert_eq!(tunnel_url("see https://www.cloudflare.com/terms"), None);
    }

    #[tokio::test]
    async fn registered_waits_for_a_connection_after_the_url() {
        let (lines, mut received) = mpsc::unbounded_channel();
        lines
            .send("INF Requesting new quick Tunnel".to_string())
            .unwrap();
        lines
            .send("INF Registered tunnel connection connIndex=0".to_string())
            .unwrap();
        lines.send(format!("INF | {URL} |")).unwrap();
        lines
            .send("INF Registered tunnel connection connIndex=1".to_string())
            .unwrap();
        assert_eq!(registered(&mut received).await.as_deref(), Ok(URL));
    }

    #[tokio::test(start_paused = true)]
    async fn registered_times_out_quoting_cloudflared() {
        let (lines, mut received) = mpsc::unbounded_channel();
        lines.send("ERR failed to dial".to_string()).unwrap();
        let error = registered(&mut received).await.unwrap_err();
        assert_eq!(error.stage, "tunnel");
        assert!(
            error.message.contains("ERR failed to dial"),
            "{}",
            error.message
        );
    }

    #[tokio::test(start_paused = true)]
    async fn published_polls_only_the_authoritative_nameservers() {
        let dns = ScriptedDns::default();
        dns.answers.lock().unwrap().extend([
            Ok(None),
            Err("timed out".to_string()),
            Ok(Some(Ipv4Addr::new(104, 16, 0, 1))),
        ]);
        let address = published(&dns, URL, DNS_INTERVAL).await.unwrap();
        assert_eq!(address, IpAddr::V4(Ipv4Addr::new(104, 16, 0, 1)));

        let asked = dns.asked.lock().unwrap();
        assert_eq!(asked[0], "NS trycloudflare.com");
        assert_eq!(asked.len(), 4, "nameservers are looked up once: {asked:?}");
        assert!(
            asked[1..]
                .iter()
                .all(|q| q == "A some-words-here.trycloudflare.com")
        );
    }

    #[tokio::test(start_paused = true)]
    async fn published_fails_its_stage_when_the_record_never_comes() {
        let error = published(&ScriptedDns::default(), URL, DNS_INTERVAL)
            .await
            .unwrap_err();
        assert_eq!(error.stage, "dns");
        assert!(error.message.contains("no A record"), "{}", error.message);
    }

    #[tokio::test]
    async fn fixed_urls_and_no_tunnel_open_nothing() {
        assert_eq!(open(&TunnelSetting::Off, 1).await, Ok(None));
        let tunnel = open(&TunnelSetting::Url("https://bot.example".into()), 1).await;
        assert_eq!(tunnel.unwrap().unwrap().url, "https://bot.example");
    }
}
