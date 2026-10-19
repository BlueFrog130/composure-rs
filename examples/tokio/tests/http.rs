//! The regression fixtures (`examples/bot/tests/common`) over a real socket: every shape must get
//! the same status and JSON from the tokio Adapter as from `composure::adapter` directly, which is
//! what the Cloudflare Adapter answers with too. Plus the example `Bot` answering Discord's Ping,
//! and the server-only commands (`/file`, `/uptime`, `/primes`) of `TokioBot`.

#[path = "../../bot/tests/common/mod.rs"]
mod common;

use std::{convert::Infallible, net::SocketAddr, time::Duration};

use bytes::Bytes;
use common::RegressionBot;
use composure_adapter_tokio::{
    EventHandler, Host, HttpTransport, InteractionHandler, TokioInteractionBot,
};
use composure_example_bot::Bot;
use composure_example_tokio::{DEFAULT_FILES_DIR, Server, TokioBot};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, body::Incoming, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};

/// Serves `handler` on a free port until the returned sender fires
async fn serve<H>(handler: H) -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>)
where
    H: InteractionHandler<Host> + EventHandler<Host> + Clone + Send + Sync + 'static,
{
    serve_with(handler, ()).await
}

/// [`serve`] with `data` in the Host
async fn serve_with<H, T>(
    handler: H,
    data: T,
) -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>)
where
    H: InteractionHandler<Host<T>> + EventHandler<Host<T>> + Clone + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let bot = TokioInteractionBot::new(&common::public_key())
        .unwrap()
        .with_handler(handler)
        .with_data(data);
    start(bot).await
}

/// Serves a configured `bot` on a free port until the returned sender fires
async fn start<H, T>(
    bot: TokioInteractionBot<H, T>,
) -> (String, oneshot::Sender<()>, tokio::task::JoinHandle<()>)
where
    H: InteractionHandler<Host<T>> + EventHandler<Host<T>> + Clone + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = oneshot::channel::<()>();
    let bot = bot.with_grace(Duration::from_secs(5)).with_shutdown(async {
        let _ = stopped.await;
    });
    let task = tokio::spawn(async move { bot.serve_on(listener).await.unwrap() });
    (url, stop, task)
}

/// A stand-in Discord API: sends (method, path, JSON body) of every request it gets down the
/// channel and answers 200 `{}`
async fn mock_discord() -> (String, mpsc::UnboundedReceiver<(String, String, Value)>) {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let (sent, received) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let sent = sent.clone();
            let service = service_fn(move |request: Request<Incoming>| {
                let sent = sent.clone();
                async move {
                    let method = request.method().to_string();
                    let path = request.uri().path().to_string();
                    let body = request.into_body().collect().await.unwrap().to_bytes();
                    let json = serde_json::from_slice(&body).unwrap_or(Value::Null);
                    let _ = sent.send((method, path, json));
                    Ok::<_, Infallible>(Response::new(Full::new(Bytes::from_static(b"{}"))))
                }
            });
            tokio::spawn(http1::Builder::new().serve_connection(TokioIo::new(stream), service));
        }
    });
    (url, received)
}

async fn post(
    url: &str,
    signature: Option<String>,
    timestamp: Option<String>,
    body: Vec<u8>,
) -> (u16, Option<Value>) {
    let mut request = reqwest::Client::new().post(url).body(body);
    if let Some(signature) = signature {
        request = request.header("X-Signature-Ed25519", signature);
    }
    if let Some(timestamp) = timestamp {
        request = request.header("X-Signature-Timestamp", timestamp);
    }
    let response = request.send().await.unwrap();
    let status = response.status().as_u16();
    let body = response.bytes().await.unwrap();
    (status, serde_json::from_slice(&body).ok())
}

#[tokio::test(flavor = "multi_thread")]
async fn regression_fixtures_over_http() {
    let (url, stop, task) = serve(RegressionBot).await;
    let mut failures = Vec::new();

    for case in common::cases() {
        let (signature, timestamp, body) = case.request();
        let (status, response) = post(&url, signature, timestamp, body).await;
        if status != case.status {
            failures.push(format!(
                "{}: expected {}, got {status}",
                case.name, case.status
            ));
            continue;
        }
        if status == 200 && response != case.response {
            failures.push(format!(
                "{}:\n  expected {}\n  got      {}",
                case.name,
                case.response.unwrap_or_default(),
                response.unwrap_or_default()
            ));
        }
    }

    stop.send(()).unwrap();
    task.await.unwrap();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_example_bot_answers_ping() {
    let (url, stop, task) = serve(Bot).await;
    let body = json!({ "id": "1", "application_id": "1", "type": 1, "token": "t", "version": 1 })
        .to_string()
        .into_bytes();
    let signature = common::sign(common::TIMESTAMP, &body);
    let (status, response) =
        post(&url, Some(signature), Some(common::TIMESTAMP.into()), body).await;
    assert_eq!(status, 200);
    assert_eq!(response, Some(json!({ "type": 1 })));
    stop.send(()).unwrap();
    task.await.unwrap();
}

/// A signed interaction of `kind` (2 command, 4 autocomplete) for `name` with `options`
async fn invoke(url: &str, kind: u8, name: &str, options: Value) -> Value {
    let body = json!({
        "id": "1", "application_id": "1", "type": kind, "token": "t", "version": 1,
        "user": { "id": "5", "username": "a", "discriminator": "0", "global_name": null, "avatar": null },
        "data": { "id": "2", "name": name, "type": 1, "options": options },
    })
    .to_string()
    .into_bytes();
    let signature = common::sign(common::TIMESTAMP, &body);
    let (status, response) = post(url, Some(signature), Some(common::TIMESTAMP.into()), body).await;
    assert_eq!(status, 200, "{response:?}");
    response.unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn server_commands() {
    let (url, stop, task) = serve_with(TokioBot, Server::new(DEFAULT_FILES_DIR)).await;
    let text = |name: &str| json!([{ "name": "name", "type": 3, "value": name }]);

    let hello = invoke(&url, 2, "file", text("hello.txt")).await;
    let content = hello["data"]["content"].as_str().unwrap();
    assert!(
        content.contains("Hello from the server's disk!"),
        "{content}"
    );
    assert_eq!(hello["data"]["flags"], 64);

    // Nothing outside the files directory, however it is spelled
    for name in ["../Cargo.toml", "..", ".env", "missing.txt"] {
        let refused = invoke(&url, 2, "file", text(name)).await;
        let description = refused["data"]["embeds"][0]["description"]
            .as_str()
            .unwrap();
        assert_eq!(description, format!("There is no file named `{name}`."));
    }

    let focused = json!([{ "name": "name", "type": 3, "value": "HE", "focused": true }]);
    let suggestions = invoke(&url, 4, "file", focused).await;
    assert_eq!(
        suggestions["data"]["choices"],
        json!([{ "name": "hello.txt", "value": "hello.txt" }])
    );

    let uptime = invoke(&url, 2, "uptime", json!([])).await;
    assert!(
        uptime["data"]["content"]
            .as_str()
            .unwrap()
            .starts_with("Up for ")
    );

    stop.send(()).unwrap();
    task.await.unwrap();
}

#[test]
fn sync_publishes_the_shared_commands_then_the_server_ones() {
    use composure::command::CommandSetDefinition;

    let shared = Bot::definitions().len();
    let names: Vec<Value> = TokioBot::definitions()[shared..]
        .iter()
        .map(|d| serde_json::to_value(&d.command).unwrap()["name"].clone())
        .collect();
    assert_eq!(names, [json!("file"), json!("uptime"), json!("primes")]);
}

#[tokio::test(flavor = "multi_thread")]
async fn primes_defers_then_edits_the_original_from_background_work() {
    let (discord, mut requests) = mock_discord().await;
    let bot = TokioInteractionBot::new(&common::public_key())
        .unwrap()
        .with_handler(TokioBot)
        .with_data(Server::new(DEFAULT_FILES_DIR))
        .with_transport(HttpTransport::new().with_api_base(&discord));
    let (url, stop, task) = start(bot).await;

    // The HTTP reply is only the deferral: Discord shows "thinking..."
    let up_to = json!([{ "name": "up_to", "type": 4, "value": 100 }]);
    assert_eq!(invoke(&url, 2, "primes", up_to).await, json!({ "type": 5 }));

    // Then Background Work edits the original response: progress, then the result
    let mut next = async || {
        tokio::time::timeout(Duration::from_secs(5), requests.recv())
            .await
            .expect("an edit within 5s")
            .unwrap()
    };
    let original = "/webhooks/1/t/messages/@original";
    let (method, path, progress) = next().await;
    assert_eq!((method.as_str(), path.as_str()), ("PATCH", original));
    assert_eq!(
        progress["content"],
        "Sieving up to 100 on a blocking thread..."
    );

    let (method, path, done) = next().await;
    assert_eq!((method.as_str(), path.as_str()), ("PATCH", original));
    let embed = &done["embeds"][0];
    let description = embed["description"].as_str().unwrap();
    assert!(
        description.starts_with(
            "25 primes up to 100, the largest 97.
Counted in "
        ),
        "{description}"
    );
    assert_eq!(embed["footer"]["text"], "composure example");

    stop.send(()).unwrap();
    task.await.unwrap();
}
