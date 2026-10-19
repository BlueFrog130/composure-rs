//! The server over a real socket: signed requests in, HTTP answers out, Background Work and
//! graceful shutdown.

use std::{
    net::SocketAddr,
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use composure::{
    Ctx, command,
    command::{EventCtx, EventHandler, HandlerError},
    commands,
    types::Snowflake,
};
use composure_adapter_tokio::{Host, MAX_BODY, TokioInteractionBot};
use ed25519_dalek::{Signer, SigningKey};
use serde_json::{Value, json};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

const SEED: [u8; 32] = [9; 32];
const TIMESTAMP: &str = "1760000000";

/// Counts handled events
#[derive(Default)]
struct Seen {
    events: AtomicUsize,
}

/// Answer pong
#[command]
fn ping() -> &'static str {
    "pong"
}

/// Panic
#[command]
fn boom() -> &'static str {
    panic!("boom")
}

/// Answer after a while
#[command]
async fn slow() -> &'static str {
    tokio::time::sleep(Duration::from_millis(300)).await;
    "done"
}

/// Read the Host's data
#[command]
fn seen(ctx: Ctx<Host<Seen>>) -> String {
    format!("{} events", ctx.state().events.load(Ordering::SeqCst))
}

/// Counts every event, slowly, so shutdown has something to wait for
struct Events;

impl EventHandler<Host<Seen>> for Events {
    async fn unknown(
        &self,
        ctx: EventCtx<'_, Host<Seen>>,
        _kind: &str,
        _data: Option<&serde_json::value::RawValue>,
    ) -> Result<(), HandlerError> {
        tokio::time::sleep(Duration::from_millis(200)).await;
        ctx.state().events.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

commands! {
    struct Bot = [ping, boom, slow, seen];
    events = Events;
}

struct Server {
    url: String,
    stop: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<(), composure_adapter_tokio::Error>>,
    host: Host<Seen>,
}

impl Server {
    async fn start() -> Self {
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (stop, stopped) = oneshot::channel();
        let key = SigningKey::from_bytes(&SEED);
        let bot = TokioInteractionBot::new(&hex::encode(key.verifying_key().as_bytes()))
            .unwrap()
            .with_handler(Bot)
            .with_data(Seen::default())
            .with_grace(Duration::from_secs(5))
            .with_shutdown(async {
                let _ = stopped.await;
            });
        let host = bot.host();
        let task = tokio::spawn(bot.serve_on(listener));
        Self {
            url,
            stop: Some(stop),
            task,
            host,
        }
    }

    async fn post(&self, path: &str, body: &[u8], signed: bool) -> reqwest::Response {
        let mut request = reqwest::Client::new()
            .post(format!("{}{path}", self.url))
            .body(body.to_vec());
        if signed {
            let key = SigningKey::from_bytes(&SEED);
            let signature = key.sign(&[TIMESTAMP.as_bytes(), body].concat());
            request = request
                .header("X-Signature-Ed25519", hex::encode(signature.to_bytes()))
                .header("X-Signature-Timestamp", TIMESTAMP);
        }
        request.send().await.unwrap()
    }

    async fn command(&self, name: &str) -> (u16, Value) {
        let body = json!({
            "id": "2", "application_id": "1", "type": 2, "token": "tok", "version": 1,
            "data": { "id": "4", "name": name, "type": 1 },
        })
        .to_string();
        let response = self.post("/", body.as_bytes(), true).await;
        let status = response.status().as_u16();
        (status, json_body(response).await)
    }

    /// Signals shutdown and waits for `serve_on` to return
    async fn stop(mut self) {
        self.stop.take().unwrap().send(()).unwrap();
        self.task.await.unwrap().unwrap();
    }
}

async fn json_body(response: reqwest::Response) -> Value {
    serde_json::from_slice(&response.bytes().await.unwrap()).unwrap_or(Value::Null)
}

#[tokio::test(flavor = "multi_thread")]
async fn answers_signed_interactions() {
    let server = Server::start().await;
    let ping = br#"{"id":"2","application_id":"1","type":1,"token":"tok","version":1}"#;
    let response = server.post("/", ping, true).await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "application/json");
    assert_eq!(json_body(response).await, json!({ "type": 1 }));

    let (status, body) = server.command("ping").await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["content"], "pong");

    // Unknown commands are ephemeral error messages, like on Workers
    let (status, body) = server.command("nope").await;
    assert_eq!(status, 200);
    assert_eq!(body["data"]["flags"], 64);
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn rejects_what_is_not_a_signed_interaction() {
    let server = Server::start().await;
    assert_eq!(server.post("/", b"{}", false).await.status(), 401);
    assert_eq!(server.post("/", b"not json", true).await.status(), 400);
    let big = vec![b' '; MAX_BODY + 1];
    assert_eq!(server.post("/", &big, true).await.status(), 413);

    let health = reqwest::get(format!("{}/health", server.url))
        .await
        .unwrap();
    assert_eq!(health.status(), 200);
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn a_panicking_handler_is_a_500_and_the_server_keeps_going() {
    let server = Server::start().await;
    assert_eq!(server.command("boom").await.0, 500);
    assert_eq!(server.command("ping").await.1["data"]["content"], "pong");
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn interactions_run_concurrently() {
    let server = Server::start().await;
    let start = Instant::now();
    let (a, b, c) = tokio::join!(
        server.command("slow"),
        server.command("slow"),
        server.command("slow")
    );
    for (status, body) in [a, b, c] {
        assert_eq!(status, 200);
        assert_eq!(body["data"]["content"], "done");
    }
    assert!(
        start.elapsed() < Duration::from_millis(800),
        "{:?}",
        start.elapsed()
    );
    server.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn events_are_acknowledged_and_shutdown_waits_for_their_handling() {
    let server = Server::start().await;
    let event = json!({
        "version": 1, "application_id": "1", "type": 1,
        "event": { "type": "SOMETHING_NEW", "timestamp": "2024-10-18T14:42:53.064834", "data": {} },
    })
    .to_string();
    let response = server.post("/events", event.as_bytes(), true).await;
    assert_eq!(response.status(), 204);
    // ACKed before the handler finishes
    assert_eq!(
        server.command("seen").await.1["data"]["content"],
        "0 events"
    );
    assert_eq!(server.post("/events", b"{}", false).await.status(), 401);

    let host = server.host.clone();
    server.stop().await;
    assert_eq!(host.events.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn host_bot_needs_a_token_and_an_application_id() {
    let key = SigningKey::from_bytes(&SEED);
    let bot = TokioInteractionBot::new(&hex::encode(key.verifying_key().as_bytes())).unwrap();
    assert!(bot.host().bot().is_none());
    let bot = bot
        .with_bot_token("token")
        .with_application_id(Snowflake::from(1u64));
    assert!(bot.host().bot().is_some());
}

#[test]
fn invalid_public_keys_fail_at_startup() {
    assert!(TokioInteractionBot::new("not hex").is_err());
}

/// What `tokio::spawn` needs: an Interaction's whole dispatch, and Background Work holding a
/// `DeferredCtx`, are `Send`
#[allow(dead_code)]
fn handler_futures_are_send(
    interaction: composure::types::Interaction,
    host: Host<Seen>,
    ctx: composure::command::DeferredCtx<Host<Seen>>,
) {
    fn send<T: Send>(_: T) {}
    send(async move { composure::command::dispatch(interaction, &Bot, &host).await });
    send(async move { ctx.sleep(Duration::from_millis(1)).await });
}
