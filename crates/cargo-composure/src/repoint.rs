//! Repoint (ADR 0018): points the Dev Application at a connected Tunnel. Only the application named
//! as the Dev Application, and only when it is the token's; only once the bot answers through the
//! tunnel, because Discord PINGs a new endpoint before accepting it.

use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use composure::{
    rest::{BotClient, EditApplication, Error, Result as RestResult, Transport},
    types::{Application, Snowflake},
};
use tokio::time::sleep;

use crate::{
    stage::{StageError, elapsed, info, within},
    tunnel::Tunnel,
};

/// The probe must get its 401 within this
const PROBE_LIMIT: Duration = Duration::from_secs(30);
const PROBE_INTERVAL: Duration = Duration::from_secs(1);
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
/// After a 400, retried once after this: past a 60 s DNS negative cache at Discord
const VERIFY_RETRY: Duration = Duration::from_secs(65);
/// A 530 is Cloudflare's own: the tunnel isn't serving yet
const NO_ROUTE: u16 = 530;

const HINT: &str = "The endpoint must answer Discord's signed PING: check that the tunnel reaches this bot and that DISCORD_PUBLIC_KEY is the Dev Application's.";

/// One unsigned POST through the tunnel, as Discord's PING will arrive; tests script it
pub trait Probe: Send + Sync {
    /// The response's status. Pinned to `address` when given.
    fn post(
        &self,
        url: &str,
        address: Option<IpAddr>,
    ) -> impl Future<Output = Result<u16, String>> + Send;
}

/// [`Probe`] over `reqwest`
pub struct HttpProbe;

impl Probe for HttpProbe {
    async fn post(&self, url: &str, address: Option<IpAddr>) -> Result<u16, String> {
        let mut client = reqwest::Client::builder().timeout(PROBE_TIMEOUT);
        if let (Some(address), Some(host)) = (address, url.strip_prefix("https://")) {
            client = client.resolve(host, SocketAddr::new(address, 443));
        }
        let response = client
            .build()
            .map_err(|e| e.to_string())?
            .post(url)
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(response.status().as_u16())
    }
}

/// A client whose 429s come back to [`patiently`], to be waited out in full
pub fn client<T: Transport>(transport: T, token: &str, application: Snowflake) -> BotClient<T> {
    BotClient::new(transport, token, application).with_max_rate_limit_wait(Duration::ZERO)
}

async fn patiently<T: Transport, R, F: Future<Output = RestResult<R>>>(
    client: &BotClient<T>,
    call: impl Fn() -> F,
) -> RestResult<R> {
    loop {
        match call().await {
            Err(Error::RateLimited(limited)) => {
                info!("rate limited; retrying in {}s", limited.retry_after);
                let wait = Duration::from_secs_f64(limited.retry_after);
                client.transport().sleep(wait).await;
            }
            result => return result,
        }
    }
}

/// The token's application, when it is the Dev Application: what Repoint and dev Sync need before
/// writing anything. Otherwise why not, for the line that says so.
pub async fn identify<T: Transport>(
    client: &BotClient<T>,
    application: Snowflake,
) -> Result<Application, String> {
    let me = patiently(client, || async {
        client.get_current_application().await?.model()
    })
    .await
    .map_err(|e| format!("reading the token's application: {e}"))?;
    // Never touch a production application: only the one named as the Dev Application
    match me.id == application {
        true => Ok(me),
        false => Err(format!(
            "application {application} is not DISCORD_TOKEN's application ({})",
            me.id
        )),
    }
}

/// Points `me`'s endpoint at the tunnel. Its Webhook Events URL follows when it has one: that URL
/// can only be a tunnel from an earlier run.
pub async fn repoint<T: Transport>(
    client: &BotClient<T>,
    probe: &impl Probe,
    me: &Application,
    tunnel: &Tunnel,
    public_key: Option<&str>,
    since: Instant,
) -> Result<(), StageError> {
    let endpoint = tunnel.url.as_str();
    within(
        "probe",
        PROBE_LIMIT,
        &format!("401 from {endpoint}"),
        async |seen| {
            loop {
                match probe.post(endpoint, tunnel.address).await {
                    Ok(401) => return Ok(()),
                    Ok(NO_ROUTE) => {
                        seen.observe("HTTP 530: Cloudflare cannot route this hostname to a tunnel")
                    }
                    Ok(status) => seen.observe(format!("HTTP {status}")),
                    Err(error) => seen.observe(error),
                }
                sleep(PROBE_INTERVAL).await;
            }
        },
    )
    .await?;
    info!("{endpoint} answers through the tunnel ({})", elapsed(since));

    let events = me
        .event_webhooks_url
        .as_deref()
        .filter(|url| !url.is_empty())
        .map(|_| format!("{endpoint}{}", composure::adapter::DEFAULT_EVENTS_PATH));
    let edit = EditApplication {
        interactions_endpoint_url: Some(Some(endpoint)),
        event_webhooks_url: events.as_deref().map(Some),
        ..Default::default()
    };

    let mut retried = false;
    loop {
        let result = patiently(client, || async {
            client.edit_current_application(&edit).await.map(|_| ())
        })
        .await;
        match result {
            Ok(()) => break,
            Err(Error::Api(error)) if error.status == 400 => {
                if public_key.is_some_and(|key| key != me.verify_key) {
                    return Err(StageError::new(
                        "repoint",
                        format!(
                            "DISCORD_PUBLIC_KEY is not the public key of application {}, so the bot rejects Discord's PING: use the Dev Application's.",
                            me.id
                        ),
                    ));
                }
                if retried {
                    return Err(StageError::new("repoint", format!("{error}\n{HINT}")));
                }
                retried = true;
                info!(
                    "Discord could not verify {endpoint}; retrying once in {}s",
                    VERIFY_RETRY.as_secs()
                );
                client.transport().sleep(VERIFY_RETRY).await;
            }
            Err(error) => return Err(StageError::new("repoint", error.to_string())),
        }
    }
    info!(
        "Dev Application {} -> {endpoint} ({})",
        me.id,
        elapsed(since)
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, sync::Mutex};

    use composure::{rest::Method, testing::MockTransport};
    use serde_json::{Value, json};

    use super::*;

    const APP: u64 = 1_100_146_565_504_565_411;

    fn app(overrides: Value) -> Application {
        let mut value = app_json();
        value
            .as_object_mut()
            .unwrap()
            .extend(overrides.as_object().unwrap().clone());
        serde_json::from_value(value).unwrap()
    }

    fn app_json() -> Value {
        json!({
            "id": APP.to_string(), "name": "dev", "description": "", "bot_public": false,
            "bot_require_code_grant": false, "verify_key": "key", "team": null,
            "summary": "", "icon": null,
        })
    }

    fn id() -> Snowflake {
        APP.to_string().parse().unwrap()
    }

    fn tunnel() -> Tunnel {
        Tunnel {
            url: "https://t.example".into(),
            address: None,
        }
    }

    /// Answers with these statuses, then 401
    #[derive(Default)]
    struct Scripted(Mutex<VecDeque<Result<u16, String>>>);

    impl Probe for Scripted {
        async fn post(&self, _: &str, _: Option<IpAddr>) -> Result<u16, String> {
            self.0.lock().unwrap().pop_front().unwrap_or(Ok(401))
        }
    }

    async fn run(
        transport: &MockTransport,
        probe: &Scripted,
        me: &Application,
        public_key: Option<&str>,
    ) -> Result<(), StageError> {
        let client = client(transport, "token", id());
        repoint(&client, probe, me, &tunnel(), public_key, Instant::now()).await
    }

    fn patches(transport: &MockTransport) -> Vec<Value> {
        transport
            .requests()
            .iter()
            .filter(|r| r.method == Method::Patch)
            .map(|r| serde_json::from_slice(r.body.as_ref().unwrap()).unwrap())
            .collect()
    }

    #[tokio::test]
    async fn identify_returns_the_dev_application() {
        let transport = MockTransport::new().respond_json(200, &app_json());
        let me = identify(&client(&transport, "t", id()), id())
            .await
            .unwrap();
        assert_eq!(me.id, id());
    }

    #[tokio::test]
    async fn another_application_is_never_touched() {
        let transport = MockTransport::new().respond_json(200, &app_json());
        let other = "1562278701016809502".parse().unwrap();
        let reason = identify(&client(&transport, "t", other), other)
            .await
            .unwrap_err();
        assert!(
            reason.contains("is not DISCORD_TOKEN's application"),
            "{reason}"
        );
        assert_eq!(transport.requests().len(), 1);
    }

    #[tokio::test]
    async fn identify_waits_out_rate_limits_in_full() {
        let transport = MockTransport::new()
            .respond_json(
                429,
                &json!({ "message": "slow", "retry_after": 30.0, "global": false }),
            )
            .respond_json(200, &app_json());
        identify(&client(&transport, "t", id()), id())
            .await
            .unwrap();
        assert_eq!(transport.sleeps(), [Duration::from_secs(30)]);
    }

    #[tokio::test(start_paused = true)]
    async fn sets_the_endpoint_once_the_bot_answers() {
        let transport = MockTransport::new();
        let probe = Scripted::default();
        probe
            .0
            .lock()
            .unwrap()
            .extend([Err("refused".to_string()), Ok(502)]);
        run(&transport, &probe, &app(json!({})), None)
            .await
            .unwrap();

        assert!(probe.0.lock().unwrap().is_empty());
        assert_eq!(
            patches(&transport),
            [json!({ "interactions_endpoint_url": "https://t.example" })]
        );
    }

    #[tokio::test]
    async fn the_events_url_follows_only_when_set() {
        let transport = MockTransport::new();
        let me = app(json!({ "event_webhooks_url": "https://old.example/events" }));
        run(&transport, &Scripted::default(), &me, None)
            .await
            .unwrap();
        assert_eq!(
            patches(&transport),
            [json!({
                "interactions_endpoint_url": "https://t.example",
                "event_webhooks_url": "https://t.example/events",
            })]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn a_bot_that_never_answers_is_not_repointed() {
        let transport = MockTransport::new();
        let probe = Scripted::default();
        probe
            .0
            .lock()
            .unwrap()
            .extend((0..100).map(|_| Ok(NO_ROUTE)));
        let error = run(&transport, &probe, &app(json!({})), None)
            .await
            .unwrap_err();
        assert_eq!(error.stage, "probe");
        assert!(error.message.contains("HTTP 530"), "{}", error.message);
        assert!(patches(&transport).is_empty());
    }

    fn rejected() -> Value {
        json!({ "code": 50035, "message": "Invalid Form Body" })
    }

    #[tokio::test]
    async fn a_mismatched_public_key_is_named_without_retrying() {
        let transport = MockTransport::new().respond_json(400, &rejected());
        let error = run(
            &transport,
            &Scripted::default(),
            &app(json!({})),
            Some("other"),
        )
        .await
        .unwrap_err();
        assert_eq!(error.stage, "repoint");
        assert!(
            error.message.contains("DISCORD_PUBLIC_KEY"),
            "{}",
            error.message
        );
        assert_eq!(patches(&transport).len(), 1);
        assert!(transport.sleeps().is_empty());
    }

    #[tokio::test]
    async fn a_400_retries_once_past_discords_negative_cache() {
        let transport = MockTransport::new().respond_json(400, &rejected());
        run(
            &transport,
            &Scripted::default(),
            &app(json!({})),
            Some("key"),
        )
        .await
        .unwrap();
        assert_eq!(patches(&transport).len(), 2);
        assert_eq!(transport.sleeps(), [VERIFY_RETRY]);
    }

    #[tokio::test]
    async fn a_second_400_fails_with_the_hint() {
        let transport = MockTransport::new()
            .respond_json(400, &rejected())
            .respond_json(400, &rejected());
        let error = run(
            &transport,
            &Scripted::default(),
            &app(json!({})),
            Some("key"),
        )
        .await
        .unwrap_err();
        assert!(error.message.contains(HINT), "{}", error.message);
        assert_eq!(patches(&transport).len(), 2);
    }
}
