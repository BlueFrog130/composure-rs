#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[macro_use]
mod log;
mod state;
#[cfg(feature = "tracing")]
pub mod tracing;
mod transport;

use std::{any::Any, borrow::Cow, cell::RefCell, future::Future, pin::Pin, rc::Rc, sync::Arc};

pub use composure::adapter::{
    DEFAULT_EVENTS_PATH, EventOutcome, InteractionOutcome, handle_event, handle_event_with_key,
    handle_interaction, handle_interaction_with_key,
};
use composure::adapter::{FilesError, SIGNATURE_HEADER, TIMESTAMP_HEADER, send_with_files, verify};
pub use composure::auth::PublicKey;
pub use composure::command::{EventHandler, HandlerError, HasData, InteractionHandler, NoHandler};
use composure::types::InteractionResponse;
pub use composure::{Credentials, CredentialsError};
use state::log_handler_error;
pub use state::{Worker, bot_client, credentials};
pub use transport::FetchTransport;
use worker::{Context, Env, Headers, Request, Response};

/// Why a request could not be validated
#[derive(Debug)]
pub enum Error {
    /// The signature is missing or invalid, or the public key is not a valid key
    ValidationError,
    /// The Workers runtime failed: missing Credentials, an unreadable request, ...
    WorkerError(worker::Error),
}

/// This crate's result
pub type Result<T> = std::result::Result<T, Error>;

/// Why a loader given to [`CloudflareInteractionBot::with_credentials`] or
/// [`with_data`](CloudflareInteractionBot::with_data) failed: any error, through `?`
pub type LoadError = Box<dyn std::error::Error>;

type Loading<T> = Pin<Box<dyn Future<Output = std::result::Result<T, LoadError>>>>;
type Loader<T> = Box<dyn Fn(Env) -> Loading<T>>;

/// Validates a request from Discord against the public key from [`credentials`]
///
/// Missing or invalid signature headers are an [`Error::ValidationError`].
///
/// # Arguments
///
/// * `env` - The environment variables for the worker
/// * `headers` - The headers of the request
/// * `body` - The body of the request
///
pub async fn validate_request(env: &Env, headers: &Headers, body: &[u8]) -> Result<()> {
    let signature = headers.get(SIGNATURE_HEADER).map_err(Error::WorkerError)?;
    let timestamp = headers.get(TIMESTAMP_HEADER).map_err(Error::WorkerError)?;
    let isolate = isolate(env, None).await?;

    if verify(
        &isolate.public_key,
        signature.as_deref(),
        timestamp.as_deref(),
        body,
    )
    .await
    {
        Ok(())
    } else {
        Err(Error::ValidationError)
    }
}

/// What the Credentials give every request of an isolate
struct Isolate {
    public_key: PublicKey,
    bot_token: Option<Arc<str>>,
}

thread_local! {
    /// The Credentials, loaded and the public key parsed (and with `web-crypto`, imported) on the
    /// first request and reused for the life of the isolate. A changed secret is a new
    /// deployment, so a new isolate.
    static ISOLATE: RefCell<Option<Rc<Isolate>>> = const { RefCell::new(None) };

    /// The Bot Data, built on the first request and shared by the isolate's requests
    static DATA: RefCell<Option<Arc<dyn Any + Send + Sync>>> = const { RefCell::new(None) };
}

/// The cached Credentials, or loaded with `loader` ([`credentials`] without one). A loader
/// failure is an [`Error::WorkerError`]; a public key that is not a valid key is an
/// [`Error::ValidationError`]. Failures are not cached: the next request loads again.
async fn isolate(env: &Env, loader: Option<&Loader<Credentials>>) -> Result<Rc<Isolate>> {
    if let Some(isolate) = ISOLATE.with_borrow(Option::clone) {
        return Ok(isolate);
    }
    let credentials = match loader {
        Some(loader) => loader(env.clone()).await,
        None => credentials(env).map_err(LoadError::from),
    }
    .map_err(|e| Error::WorkerError(worker::Error::RustError(e.to_string())))?;
    let public_key = PublicKey::from_hex(credentials.public_key())
        .await
        .map_err(|_| Error::ValidationError)?;
    let isolate = Rc::new(Isolate {
        public_key,
        bot_token: credentials.bot_token().map(Arc::from),
    });
    ISOLATE.set(Some(isolate.clone()));
    Ok(isolate)
}

/// The cached Bot Data, or built with `loader`; a failure is not cached
async fn data<T: Send + Sync + 'static>(
    env: &Env,
    loader: &Loader<T>,
) -> std::result::Result<Arc<T>, LoadError> {
    if let Some(data) = DATA.with_borrow(Option::clone)
        && let Ok(data) = data.downcast::<T>()
    {
        return Ok(data);
    }
    let data = Arc::new(loader(env.clone()).await?);
    DATA.set(Some(data.clone()));
    Ok(data)
}

/// Interaction bot for Cloudflare. The handler receives a [`Worker`] (the worker's [`Env`],
/// [`Context`], and Bot Data) as its State. Webhook Events on the events path (default
/// `/events`) go to the same handler's [`EventHandler`] implementation.
///
/// The [`Credentials`] come from the Worker's vars and secrets ([`credentials`]) unless
/// [`with_credentials`](Self::with_credentials) loads them another way; both they and the Bot
/// Data ([`with_data`](Self::with_data)) are loaded on an isolate's first request and reused.
pub struct CloudflareInteractionBot<H = NoHandler, T = ()> {
    req: Request,
    env: Env,
    ctx: Context,
    handler: H,
    events_path: Cow<'static, str>,
    credentials: Option<Loader<Credentials>>,
    data: Loader<T>,
}

impl CloudflareInteractionBot {
    /// Creates a new Cloudflare interaction bot. Without [`with_handler`](Self::with_handler)
    /// every interaction gets the [`InteractionHandler`] defaults.
    pub fn new(req: Request, env: Env, ctx: Context) -> Self {
        Self {
            req,
            env,
            ctx,
            handler: NoHandler,
            events_path: Cow::Borrowed(DEFAULT_EVENTS_PATH),
            credentials: None,
            data: Box::new(|_| Box::pin(async { Ok(()) })),
        }
    }
}

impl<H, T> CloudflareInteractionBot<H, T> {
    /// Handles interactions with `handler`: an `#[app]`, an App `with_fallback`, or any
    /// [`InteractionHandler`]
    pub fn with_handler<U>(self, handler: U) -> CloudflareInteractionBot<U, T> {
        CloudflareInteractionBot {
            req: self.req,
            env: self.env,
            ctx: self.ctx,
            handler,
            events_path: self.events_path,
            credentials: self.credentials,
            data: self.data,
        }
    }

    /// Loads the [`Credentials`] with `loader` instead of [`credentials`] (the conventional
    /// names): other names, a JSON var, the Secrets Store, ... Runs on an isolate's first request;
    /// an error is logged, answered with 500, and retried on the next request.
    ///
    /// ```ignore
    /// CloudflareInteractionBot::new(req, env, ctx)
    ///     .with_credentials(async |env: Env| {
    ///         let token = env.secret_store("BOT_TOKEN")?.get().await?;
    ///         Ok(Credentials::new(&env.var("PUBLIC_KEY")?.to_string())?
    ///             .with_bot_token(token.unwrap_or_default()))
    ///     })
    /// ```
    pub fn with_credentials<F, Fut>(mut self, loader: F) -> Self
    where
        F: Fn(Env) -> Fut + 'static,
        Fut: Future<Output = std::result::Result<Credentials, LoadError>> + 'static,
    {
        self.credentials = Some(Box::new(move |env| Box::pin(loader(env))));
        self
    }

    /// Gives handlers Bot Data built by `loader` from the Worker's [`Env`] (`ctx.state().data()`,
    /// [`HasData`]): parsed vars, settings read from KV, ... Runs on an isolate's first request
    /// and is shared by its requests, so it must be `Send + Sync`; keep bindings (KV, D1) on
    /// `Env`. An error is logged, answered with 500, and retried on the next request.
    ///
    /// ```ignore
    /// CloudflareInteractionBot::new(req, env, ctx)
    ///     .with_handler(Bot)
    ///     .with_data(async |env: Env| Ok(Config { greeting: env.var("GREETING")?.to_string() }))
    /// ```
    pub fn with_data<U, F, Fut>(self, loader: F) -> CloudflareInteractionBot<H, U>
    where
        F: Fn(Env) -> Fut + 'static,
        Fut: Future<Output = std::result::Result<U, LoadError>> + 'static,
    {
        CloudflareInteractionBot {
            req: self.req,
            env: self.env,
            ctx: self.ctx,
            handler: self.handler,
            events_path: self.events_path,
            credentials: self.credentials,
            data: Box::new(move |env| Box::pin(loader(env))),
        }
    }

    /// Serves Webhook Events on `path` instead of `/events`; set the app's Webhook Events URL to
    /// match. Every other path serves Interactions.
    pub fn with_events_path(mut self, path: impl Into<Cow<'static, str>>) -> Self {
        self.events_path = path.into();
        self
    }
}

impl<T, H> CloudflareInteractionBot<H, T>
where
    T: Send + Sync + 'static,
    H: InteractionHandler<Worker<T>> + EventHandler<Worker<T>> + Send + 'static,
{
    /// Validates and handles the request. Every outcome is an HTTP response:
    /// bad signature is 401, unparseable body is 400, Credentials or Bot Data that fail to load
    /// are 500, handler errors are logged and answered with an ephemeral error message.
    /// Responses with files are sent through the callback endpoint and the request is answered
    /// with 202. Webhook Events are acknowledged with 204 and handled as Background Work.
    pub async fn process(self) -> worker::Result<Response> {
        let Self {
            mut req,
            env,
            ctx,
            handler,
            events_path,
            credentials,
            data: data_loader,
        } = self;
        let bytes = match req.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                error!("failed to read request body", error = e);
                return Response::error("Bad request", 400);
            }
        };

        let isolate = match isolate(&env, credentials.as_ref()).await {
            Ok(isolate) => isolate,
            Err(Error::WorkerError(e)) => {
                error!("failed to load the credentials", error = e);
                return Response::error("Internal error", 500);
            }
            Err(Error::ValidationError) => {
                error!("the credentials' public key is not a valid Ed25519 key");
                return Response::error("Internal error", 500);
            }
        };
        let data = match data(&env, &data_loader).await {
            Ok(data) => data,
            Err(e) => {
                error!("failed to load the bot data", error = e);
                return Response::error("Internal error", 500);
            }
        };
        let state = Worker::from_parts(env, ctx, isolate.bot_token.clone(), data);
        let headers = req.headers();
        let signature = headers.get(SIGNATURE_HEADER).ok().flatten();
        let timestamp = headers.get(TIMESTAMP_HEADER).ok().flatten();

        if req.path() == events_path {
            let outcome = handle_event_with_key(
                &isolate.public_key,
                signature.as_deref(),
                timestamp.as_deref(),
                &bytes,
                handler,
                &state,
            )
            .await;
            return match outcome {
                EventOutcome::Unauthorized => {
                    warn!("event signature validation failed");
                    Response::error("Validation failed", 401)
                }
                EventOutcome::BadRequest(e) => {
                    error!("failed to parse event", error = e);
                    Response::error("Bad request", 400)
                }
                EventOutcome::Acknowledged => {
                    let headers = Headers::new();
                    headers.set("Content-Type", "application/json")?;
                    Ok(Response::empty()?.with_status(204).with_headers(headers))
                }
            };
        }

        let outcome = handle_interaction_with_key(
            &isolate.public_key,
            signature.as_deref(),
            timestamp.as_deref(),
            &bytes,
            &handler,
            &state,
        )
        .await;

        match outcome {
            InteractionOutcome::Unauthorized => {
                warn!("interaction signature validation failed");
                Response::error("Validation failed", 401)
            }
            InteractionOutcome::BadRequest(e) => {
                error!("failed to parse interaction", error = e);
                Response::error("Bad request", 400)
            }
            InteractionOutcome::Respond(response) => reply(&bytes, &response).await,
            InteractionOutcome::HandlerFailed { error, response } => {
                log_handler_error(&error);
                reply(&bytes, &response).await
            }
        }
    }
}

/// The response as the HTTP reply, or, when it carries files (multipart, which the reply cannot
/// be), through the callback endpoint followed by an empty 202
async fn reply(body: &[u8], response: &InteractionResponse) -> worker::Result<Response> {
    if response.files().is_empty() {
        return Response::from_json(response);
    }
    match send_with_files(FetchTransport, body, response).await {
        Ok(()) => Ok(Response::empty()?.with_status(202)),
        Err(e @ FilesError::MissingCallback) => {
            error!("failed to send response with files", error = e);
            Response::error("Bad request", 400)
        }
        Err(e) => {
            error!("failed to send response with files", error = e);
            Response::error("Internal error", 500)
        }
    }
}
