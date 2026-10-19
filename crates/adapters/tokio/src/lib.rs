#![doc = include_str!("../README.md")]

mod host;
mod transport;

use std::{
    borrow::Cow, convert::Infallible, fmt, future::Future, io, net::SocketAddr, pin::Pin,
    sync::Arc, time::Duration,
};

use bytes::Bytes;
pub use composure::adapter::DEFAULT_EVENTS_PATH;
pub use composure::command::{EventHandler, HandlerError, InteractionHandler, NoHandler};
use composure::{
    adapter::{
        APPLICATION_ID_VAR, BOT_TOKEN_VAR, EventOutcome, FilesError, InteractionOutcome,
        PUBLIC_KEY_VAR, SIGNATURE_HEADER, TIMESTAMP_HEADER, handle_event_with_key,
        handle_interaction_with_key, send_with_files,
    },
    auth::PublicKey,
    types::{InteractionResponse, Snowflake},
};
pub use host::Host;
use http_body_util::{BodyExt, Full, Limited};
use hyper::{
    Method, Request, Response, StatusCode, body::Incoming, header, server::conn::http1,
    service::service_fn,
};
use hyper_util::{rt::TokioIo, server::graceful::GracefulShutdown};
use tokio::net::TcpListener;
pub use transport::HttpTransport;

/// Requests with a bigger body are answered with 413; Discord's are a few KB
pub const MAX_BODY: usize = 1024 * 1024;

/// Path answering `GET` with 200, for load balancers and orchestrators
pub const HEALTH_PATH: &str = "/health";

/// Port served on when `PORT` is not set
pub const DEFAULT_PORT: u16 = 8080;

/// How long shutdown waits for in-flight requests and Background Work by default
pub const DEFAULT_GRACE: Duration = Duration::from_secs(30);

/// Why the bot could not be configured or served
#[derive(Debug)]
pub enum Error {
    /// A required environment variable is not set
    MissingVar(&'static str),

    /// An environment variable is set but unusable
    InvalidVar { name: &'static str, reason: String },

    /// The public key is not a valid hex Ed25519 key
    InvalidPublicKey,

    /// Binding or accepting on the listener failed
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingVar(name) => write!(f, "{name} is not set"),
            Self::InvalidVar { name, reason } => write!(f, "{name} is invalid: {reason}"),
            Self::InvalidPublicKey => f.write_str("the public key is not a valid Ed25519 key"),
            Self::Io(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

type Shutdown = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Interaction bot on a hosted server: an HTTP server on tokio answering Discord's signed
/// requests, each Interaction on its own task (ADR 0012). The handler receives a [`Host`] (your
/// data plus the bot token, Transport, and Background Work) as its State. Webhook Events on the
/// events path (default `/events`) go to the same handler's [`EventHandler`] implementation;
/// `GET /health` answers 200. Plain HTTP: put it behind a TLS-terminating proxy or tunnel.
///
/// ```ignore
/// #[tokio::main]
/// async fn main() -> Result<(), composure_adapter_tokio::Error> {
///     TokioInteractionBot::from_env()?
///         .with_handler(Bot)
///         .serve()
///         .await
/// }
/// ```
pub struct TokioInteractionBot<H = NoHandler, T = ()> {
    public_key: PublicKey,
    host: Host<T>,
    handler: H,
    addr: SocketAddr,
    events_path: Cow<'static, str>,
    grace: Duration,
    shutdown: Option<Shutdown>,
}

impl TokioInteractionBot {
    /// Configured from the environment: `DISCORD_PUBLIC_KEY` (required), `DISCORD_TOKEN` and
    /// `DISCORD_APPLICATION_ID` (optional, for REST), serving on `0.0.0.0:$PORT` (8080 without
    /// `PORT`). Fails at once on a missing or invalid value.
    pub fn from_env() -> Result<Self, Error> {
        let public_key =
            std::env::var(PUBLIC_KEY_VAR).map_err(|_| Error::MissingVar(PUBLIC_KEY_VAR))?;
        let mut bot = Self::new(&public_key)?;
        if let Some(token) = var(BOT_TOKEN_VAR)? {
            bot = bot.with_bot_token(token);
        }
        if let Some(id) = var(APPLICATION_ID_VAR)? {
            let id = id.parse::<Snowflake>().map_err(|_| Error::InvalidVar {
                name: APPLICATION_ID_VAR,
                reason: "not an id".into(),
            })?;
            bot = bot.with_application_id(id);
        }
        if let Some(port) = var("PORT")? {
            let port = port.parse::<u16>().map_err(|e| Error::InvalidVar {
                name: "PORT",
                reason: e.to_string(),
            })?;
            bot = bot.bind(([0, 0, 0, 0], port));
        }
        Ok(bot)
    }

    /// Configured in code with the application's hex public key; no bot token, serving on
    /// `0.0.0.0:8080`
    pub fn new(public_key: &str) -> Result<Self, Error> {
        // Natively the key is parsed by ed25519-dalek; the future is ready at once
        let public_key = futures::executor::block_on(PublicKey::from_hex(public_key))
            .map_err(|_| Error::InvalidPublicKey)?;
        Ok(Self {
            public_key,
            host: Host::new(None, None),
            handler: NoHandler,
            addr: SocketAddr::from(([0, 0, 0, 0], DEFAULT_PORT)),
            events_path: Cow::Borrowed(DEFAULT_EVENTS_PATH),
            grace: DEFAULT_GRACE,
            shutdown: None,
        })
    }
}

/// An environment variable, `None` when unset or empty
fn var(name: &'static str) -> Result<Option<String>, Error> {
    match std::env::var(name) {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(Error::InvalidVar {
            name,
            reason: "not unicode".into(),
        }),
    }
}

impl<H, T> TokioInteractionBot<H, T> {
    /// Handles interactions with `handler`: an `#[app]`, an App `with_fallback`, or any
    /// [`InteractionHandler`]
    pub fn with_handler<U>(self, handler: U) -> TokioInteractionBot<U, T> {
        TokioInteractionBot {
            public_key: self.public_key,
            host: self.host,
            handler,
            addr: self.addr,
            events_path: self.events_path,
            grace: self.grace,
            shutdown: self.shutdown,
        }
    }

    /// Gives handlers `data` through the [`Host`] (`ctx.state().data()`, or `Deref`): a database
    /// pool, config, ... Shared between Interactions, so it must be `Send + Sync`.
    pub fn with_data<U>(self, data: U) -> TokioInteractionBot<H, U> {
        TokioInteractionBot {
            public_key: self.public_key,
            host: self.host.with_data(data),
            handler: self.handler,
            addr: self.addr,
            events_path: self.events_path,
            grace: self.grace,
            shutdown: self.shutdown,
        }
    }

    /// The bot token for `ctx.bot()` and [`Host::bot`], without the `Bot ` prefix
    pub fn with_bot_token(mut self, token: impl Into<String>) -> Self {
        self.host.core_mut().bot_token = Some(token.into());
        self
    }

    /// The application id for [`Host::bot`] (Interactions carry their own)
    pub fn with_application_id(mut self, id: Snowflake) -> Self {
        self.host.core_mut().application_id = Some(id);
        self
    }

    /// Sends REST requests through `transport`, e.g. one sharing the server's `reqwest::Client`
    pub fn with_transport(mut self, transport: HttpTransport) -> Self {
        self.host.core_mut().transport = transport;
        self
    }

    /// The address [`serve`](Self::serve) binds
    pub fn bind(mut self, addr: impl Into<SocketAddr>) -> Self {
        self.addr = addr.into();
        self
    }

    /// Serves Webhook Events on `path` instead of `/events`; set the app's Webhook Events URL to
    /// match. Every other path serves Interactions.
    pub fn with_events_path(mut self, path: impl Into<Cow<'static, str>>) -> Self {
        self.events_path = path.into();
        self
    }

    /// Shuts down when `signal` completes instead of on Ctrl-C / SIGTERM
    pub fn with_shutdown(mut self, signal: impl Future<Output = ()> + Send + 'static) -> Self {
        self.shutdown = Some(Box::pin(signal));
        self
    }

    /// How long shutdown waits for in-flight requests and Background Work (30s by default)
    pub fn with_grace(mut self, grace: Duration) -> Self {
        self.grace = grace;
        self
    }

    /// The State handlers will get, for REST and tasks outside Interactions ([`Host::bot`],
    /// [`Host::spawn`]). Take it after configuring the token, application id, and Transport.
    pub fn host(&self) -> Host<T> {
        self.host.clone()
    }
}

/// What every connection task shares
struct App<H, T> {
    public_key: PublicKey,
    host: Host<T>,
    handler: H,
    events_path: Cow<'static, str>,
}

impl<H, T> TokioInteractionBot<H, T>
where
    H: InteractionHandler<Host<T>> + EventHandler<Host<T>> + Clone + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    /// Binds the configured address and serves until shutdown. See [`serve_on`](Self::serve_on).
    pub async fn serve(self) -> Result<(), Error> {
        let listener = TcpListener::bind(self.addr).await?;
        self.serve_on(listener).await
    }

    /// Serves on `listener` until the shutdown signal (Ctrl-C / SIGTERM, or
    /// [`with_shutdown`](Self::with_shutdown)), then stops accepting and waits up to the grace
    /// period for in-flight requests and Background Work.
    ///
    /// Every outcome is an HTTP response: bad signature is 401, unparseable body is 400, a body
    /// over [`MAX_BODY`] is 413, handler errors are answered with an ephemeral error message and a
    /// panicking handler with 500. Responses with files are sent through the callback endpoint
    /// and the request is answered with 202. Webhook Events are acknowledged with 204 and handled
    /// as Background Work.
    pub async fn serve_on(self, listener: TcpListener) -> Result<(), Error> {
        let shutdown = self.shutdown.unwrap_or_else(|| Box::pin(shutdown_signal()));
        let tracker = self.host.tracker().clone();
        let app = Arc::new(App {
            public_key: self.public_key,
            host: self.host,
            handler: self.handler,
            events_path: self.events_path,
        });
        let graceful = GracefulShutdown::new();
        tracing::info!(addr = %listener.local_addr()?, "serving interactions");

        tokio::pin!(shutdown);
        loop {
            let stream = tokio::select! {
                accepted = listener.accept() => match accepted {
                    Ok((stream, _)) => stream,
                    Err(error) => {
                        // Per-connection failures (resets, fd limits): keep accepting
                        tracing::warn!(%error, "accepting a connection failed");
                        continue;
                    }
                },
                () = &mut shutdown => break,
            };
            let app = app.clone();
            let service = service_fn(move |request| {
                let app = app.clone();
                async move { Ok::<_, Infallible>(respond(app, request).await) }
            });
            let connection = http1::Builder::new().serve_connection(TokioIo::new(stream), service);
            let connection = graceful.watch(connection);
            tokio::spawn(async move {
                if let Err(error) = connection.await {
                    tracing::debug!(%error, "connection closed with an error");
                }
            });
        }

        drop(listener);
        tracing::info!("shutting down; waiting for requests and background work");
        tracker.close();
        let drained = tokio::time::timeout(self.grace, async {
            graceful.shutdown().await;
            tracker.wait().await;
        })
        .await;
        match drained {
            Ok(()) => tracing::info!("shut down"),
            Err(_) => tracing::warn!(
                tasks = tracker.len(),
                "grace period elapsed; dropping unfinished work"
            ),
        }
        Ok(())
    }
}

/// Ctrl-C, or SIGTERM on Unix (what orchestrators send)
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "listening for Ctrl-C failed");
            std::future::pending::<()>().await;
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => drop(signal.recv().await),
            Err(error) => {
                tracing::error!(%error, "listening for SIGTERM failed");
                std::future::pending::<()>().await
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}

type HttpResponse = Response<Full<Bytes>>;

async fn respond<H, T>(app: Arc<App<H, T>>, request: Request<Incoming>) -> HttpResponse
where
    H: InteractionHandler<Host<T>> + EventHandler<Host<T>> + Clone + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let path = request.uri().path();
    if path == HEALTH_PATH && request.method() == Method::GET {
        return text(StatusCode::OK, "ok");
    }
    let is_event = path == app.events_path;
    let headers = request.headers();
    let signature = header_str(headers, SIGNATURE_HEADER);
    let timestamp = header_str(headers, TIMESTAMP_HEADER);

    let body = match Limited::new(request.into_body(), MAX_BODY).collect().await {
        Ok(body) => body.to_bytes(),
        Err(error) if error.is::<http_body_util::LengthLimitError>() => {
            tracing::warn!(limit = MAX_BODY, "request body too large");
            return text(StatusCode::PAYLOAD_TOO_LARGE, "Payload too large");
        }
        Err(error) => {
            tracing::error!(%error, "failed to read request body");
            return text(StatusCode::BAD_REQUEST, "Bad request");
        }
    };

    if is_event {
        let outcome = handle_event_with_key(
            &app.public_key,
            signature.as_deref(),
            timestamp.as_deref(),
            &body,
            app.handler.clone(),
            &app.host,
        )
        .await;
        return match outcome {
            EventOutcome::Unauthorized => {
                tracing::warn!("event signature validation failed");
                text(StatusCode::UNAUTHORIZED, "Validation failed")
            }
            EventOutcome::BadRequest(error) => {
                tracing::error!(%error, "failed to parse event");
                text(StatusCode::BAD_REQUEST, "Bad request")
            }
            EventOutcome::Acknowledged => Response::builder()
                .status(StatusCode::NO_CONTENT)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::default())
                .expect("valid response"),
        };
    }

    // Its own task, so a panicking handler is answered with 500 instead of dropping the
    // connection
    let task = tokio::spawn({
        let app = app.clone();
        let body = body.clone();
        async move {
            handle_interaction_with_key(
                &app.public_key,
                signature.as_deref(),
                timestamp.as_deref(),
                &body,
                &app.handler,
                &app.host,
            )
            .await
        }
    });
    let outcome = match task.await {
        Ok(outcome) => outcome,
        Err(error) => {
            tracing::error!(%error, "handler panicked");
            return text(StatusCode::INTERNAL_SERVER_ERROR, "Internal error");
        }
    };
    match outcome {
        InteractionOutcome::Unauthorized => {
            tracing::warn!("interaction signature validation failed");
            text(StatusCode::UNAUTHORIZED, "Validation failed")
        }
        InteractionOutcome::BadRequest(error) => {
            tracing::error!(%error, "failed to parse interaction");
            text(StatusCode::BAD_REQUEST, "Bad request")
        }
        // Composure logged a handler's error inside the Interaction's span already
        InteractionOutcome::Respond(response)
        | InteractionOutcome::HandlerFailed { response, .. } => {
            reply(&app.host, &body, &response).await
        }
    }
}

/// The response as JSON, or, when it carries files (multipart, which the reply cannot be),
/// through the callback endpoint followed by an empty 202
async fn reply<T>(host: &Host<T>, body: &[u8], response: &InteractionResponse) -> HttpResponse {
    if response.files().is_empty() {
        return match serde_json::to_vec(response) {
            Ok(json) => Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(json)))
                .expect("valid response"),
            Err(error) => {
                tracing::error!(%error, "failed to serialize the response");
                text(StatusCode::INTERNAL_SERVER_ERROR, "Internal error")
            }
        };
    }
    match send_with_files(host.transport(), body, response).await {
        Ok(()) => Response::builder()
            .status(StatusCode::ACCEPTED)
            .body(Full::default())
            .expect("valid response"),
        Err(error @ FilesError::MissingCallback) => {
            tracing::error!(%error, "failed to send response with files");
            text(StatusCode::BAD_REQUEST, "Bad request")
        }
        Err(error) => {
            tracing::error!(%error, "failed to send response with files");
            text(StatusCode::INTERNAL_SERVER_ERROR, "Internal error")
        }
    }
}

fn header_str(headers: &hyper::HeaderMap, name: &str) -> Option<String> {
    Some(headers.get(name)?.to_str().ok()?.to_owned())
}

fn text(status: StatusCode, body: &'static str) -> HttpResponse {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .expect("valid response")
}
