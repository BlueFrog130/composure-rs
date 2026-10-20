//! A [`tracing`](https://docs.rs/tracing) subscriber for Workers (`tracing` feature). Each event is one console
//! call (ERROR `console.error`, WARN `console.warn`, INFO `console.log`, DEBUG and TRACE
//! `console.debug`) with an object, so Workers Logs indexes every field: `level`, `target`,
//! `spans` (names, outermost first), then the fields of each enclosing span and of the event
//! (`message` included), inner ones winning.
//!
//! Composure's spans put `interaction_id`, `interaction_type`, `name`, `guild_id`, `channel_id`
//! and `user_id` on everything logged while handling an Interaction (its Background Work
//! included), `event_type` while handling a Webhook Event, and `method` / `route` during REST
//! calls. Handler errors are logged inside those spans with `error` and `error_kind` fields
//! (ERROR for failures, WARN for unknown commands and invalid Options, DEBUG for user errors).
//!
//! Install it once per isolate, from `#[event(start)]`:
//!
//! ```ignore
//! #[event(start)]
//! fn start() {
//!     composure_adapter_cloudflare::tracing::init();
//! }
//! ```
//!
//! [`ConsoleLayer`] composes with other `tracing-subscriber` layers and filters for more control.

use std::fmt;

use ::tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id, Record},
};
use tracing_subscriber::{Layer, layer::Context, prelude::*, registry::LookupSpan};

pub use tracing_subscriber::filter::LevelFilter;

/// Installs [`ConsoleLayer`] showing INFO and above as the global subscriber. Does nothing if
/// one is already installed.
pub fn init() {
    init_with_level(LevelFilter::INFO);
}

/// [`init`] showing `level` and above (`LevelFilter::DEBUG` includes Composure's REST responses
/// and retries)
pub fn init_with_level(level: LevelFilter) {
    let _ = tracing_subscriber::registry()
        .with(level)
        .with(ConsoleLayer::new())
        .try_init();
}

/// Writes events to the Workers console as objects; see the [module docs](self)
pub struct ConsoleLayer {
    write: fn(&Line),
}

impl ConsoleLayer {
    /// A layer writing to the console as objects
    pub fn new() -> Self {
        Self { write: console }
    }
}

impl Default for ConsoleLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for ConsoleLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut fields = Fields::new();
        attrs.record(&mut Visitor(&mut fields));
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SpanFields(fields));
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id)
            && let Some(SpanFields(fields)) = span.extensions_mut().get_mut::<SpanFields>()
        {
            values.record(&mut Visitor(fields));
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut fields = Fields::new();
        let mut spans = Vec::new();
        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                spans.push(span.name());
                if let Some(SpanFields(span_fields)) = span.extensions().get::<SpanFields>() {
                    for (name, value) in span_fields {
                        set(&mut fields, name, value.clone());
                    }
                }
            }
        }
        event.record(&mut Visitor(&mut fields));
        let metadata = event.metadata();
        (self.write)(&Line {
            level: *metadata.level(),
            target: metadata.target(),
            spans,
            fields,
        });
    }
}

/// One event, ready to write
#[derive(Debug, Clone, PartialEq)]
struct Line {
    level: Level,
    target: &'static str,
    spans: Vec<&'static str>,
    fields: Fields,
}

type Fields = Vec<(&'static str, Value)>;

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Str(String),
    I64(i64),
    U64(u64),
    F64(f64),
    Bool(bool),
}

/// A span's recorded fields, kept in its extensions
struct SpanFields(Fields);

fn set(fields: &mut Fields, name: &'static str, value: Value) {
    match fields.iter_mut().find(|(key, _)| *key == name) {
        Some((_, old)) => *old = value,
        None => fields.push((name, value)),
    }
}

struct Visitor<'a>(&'a mut Fields);

impl Visit for Visitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        set(self.0, field.name(), Value::Str(format!("{value:?}")));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        set(self.0, field.name(), Value::Str(value.to_owned()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        set(self.0, field.name(), Value::I64(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        set(self.0, field.name(), Value::U64(value));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        set(self.0, field.name(), Value::F64(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        set(self.0, field.name(), Value::Bool(value));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        set(self.0, field.name(), Value::Str(value.to_string()));
    }
}

#[cfg(target_arch = "wasm32")]
fn console(line: &Line) {
    use worker::{
        js_sys::{Array, Object, Reflect},
        wasm_bindgen::JsValue,
        web_sys::console,
    };

    let object = Object::new();
    let set = |key: &str, value: JsValue| {
        let _ = Reflect::set(&object, &JsValue::from_str(key), &value);
    };
    set("level", JsValue::from_str(line.level.as_str()));
    set("target", JsValue::from_str(line.target));
    if !line.spans.is_empty() {
        let spans: Array = line
            .spans
            .iter()
            .map(|name| JsValue::from_str(name))
            .collect();
        set("spans", spans.into());
    }
    for (name, value) in &line.fields {
        let value = match value {
            Value::Str(s) => JsValue::from_str(s),
            Value::I64(n) => JsValue::from_f64(*n as f64),
            Value::U64(n) => JsValue::from_f64(*n as f64),
            Value::F64(n) => JsValue::from_f64(*n),
            Value::Bool(b) => JsValue::from_bool(*b),
        };
        set(name, value);
    }
    match line.level {
        Level::ERROR => console::error_1(&object),
        Level::WARN => console::warn_1(&object),
        Level::INFO => console::log_1(&object),
        _ => console::debug_1(&object),
    }
}

/// Off Workers (tests, local tools) as text on stderr
#[cfg(not(target_arch = "wasm32"))]
fn console(line: &Line) {
    eprintln!("{line}");
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:>5} {}", self.level, self.target)?;
        if !self.spans.is_empty() {
            write!(f, "{{{}}}", self.spans.join(">"))?;
        }
        f.write_str(":")?;
        if let Some((_, Value::Str(message))) =
            self.fields.iter().find(|(name, _)| *name == "message")
        {
            write!(f, " {message}")?;
        }
        for (name, value) in &self.fields {
            match (*name, value) {
                ("message", Value::Str(_)) => {}
                (name, Value::Str(s)) => write!(f, " {name}={s:?}")?,
                (name, Value::I64(n)) => write!(f, " {name}={n}")?,
                (name, Value::U64(n)) => write!(f, " {name}={n}")?,
                (name, Value::F64(n)) => write!(f, " {name}={n}")?,
                (name, Value::Bool(b)) => write!(f, " {name}={b}")?,
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use composure::{
        command::{HandlerError, InteractionHandler, dispatch},
        types::{ApplicationCommandInteraction, Interaction, InteractionResponse},
    };
    use futures::executor::block_on;
    use serde_json::json;

    use super::*;

    thread_local! {
        static LINES: RefCell<Vec<Line>> = const { RefCell::new(Vec::new()) };
    }

    fn capture(line: &Line) {
        LINES.with_borrow_mut(|lines| lines.push(line.clone()));
    }

    /// The lines written while `f` runs with a capturing [`ConsoleLayer`]
    fn lines(f: impl FnOnce()) -> Vec<Line> {
        let subscriber = tracing_subscriber::registry().with(ConsoleLayer { write: capture });
        ::tracing::subscriber::with_default(subscriber, f);
        LINES.take()
    }

    fn field<'a>(line: &'a Line, name: &str) -> Option<&'a Value> {
        line.fields
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value)
    }

    /// The one line logged with `message`
    fn message<'a>(lines: &'a [Line], message: &str) -> &'a Line {
        let [line] = &lines
            .iter()
            .filter(|line| field(line, "message") == str(message).as_ref())
            .collect::<Vec<_>>()[..]
        else {
            panic!("{lines:?}")
        };
        line
    }

    fn str(s: &str) -> Option<Value> {
        Some(Value::Str(s.to_owned()))
    }

    #[test]
    fn merges_span_and_event_fields() {
        let lines = lines(|| {
            let outer = ::tracing::info_span!("outer", a = 1, b = "outer");
            let _outer = outer.enter();
            let inner = ::tracing::info_span!("inner", b = "inner", c = ::tracing::field::Empty);
            inner.record("c", true);
            let _inner = inner.enter();
            ::tracing::warn!(d = %"display", "hello {}", 5);
        });

        let [line] = &lines[..] else {
            panic!("{lines:?}")
        };
        assert_eq!(line.level, Level::WARN);
        assert_eq!(line.spans, ["outer", "inner"]);
        assert_eq!(field(line, "a"), Some(&Value::I64(1)));
        assert_eq!(field(line, "b").cloned(), str("inner"));
        assert_eq!(field(line, "c"), Some(&Value::Bool(true)));
        assert_eq!(field(line, "d").cloned(), str("display"));
        assert_eq!(field(line, "message").cloned(), str("hello 5"));
        assert_eq!(
            line.to_string(),
            format!(
                " WARN {}{{outer>inner}}: hello 5 a=1 b=\"inner\" c=true d=\"display\"",
                module_path!()
            )
        );
    }

    struct Logging;

    impl<S: Sync> InteractionHandler<S> for Logging {
        async fn command(
            &self,
            _command: ApplicationCommandInteraction,
            _state: &S,
        ) -> Result<InteractionResponse, HandlerError> {
            ::tracing::info!("handled");
            Ok(InteractionResponse::respond_with_message("ok".to_string()))
        }
    }

    #[test]
    fn handler_events_carry_the_interaction() {
        let interaction: Interaction = serde_json::from_value(json!({
            "id": "1100173248714518568",
            "application_id": "1052322265397739523",
            "type": 2,
            "token": "token",
            "version": 1,
            "guild_id": "1052322265397739524",
            "member": {
                "user": { "id": "1052322265397739525", "username": "u", "discriminator": "0" },
                "roles": [],
                "joined_at": "2023-01-01T00:00:00Z",
                "flags": 0,
                "deaf": false,
                "mute": false,
            },
            "data": { "id": "1052358444704862218", "name": "ping", "type": 1 },
        }))
        .unwrap();

        let lines = lines(|| {
            block_on(dispatch(interaction, &Logging, &())).unwrap();
        });

        // Composure's own DEBUG "received" / "answered" lines frame the handler's
        let line = message(&lines, "handled");
        assert_eq!(line.spans, ["interaction"]);
        assert_eq!(
            field(line, "interaction_id").cloned(),
            str("1100173248714518568")
        );
        assert_eq!(field(line, "interaction_type").cloned(), str("command"));
        assert_eq!(field(line, "name").cloned(), str("ping"));
        assert_eq!(field(line, "guild_id").cloned(), str("1052322265397739524"));
        assert_eq!(field(line, "user_id").cloned(), str("1052322265397739525"));
        // The token is never recorded
        assert!(!line.to_string().contains("token"));
    }

    struct Failing;

    impl<S: Sync> InteractionHandler<S> for Failing {
        async fn command(
            &self,
            _command: ApplicationCommandInteraction,
            _state: &S,
        ) -> Result<InteractionResponse, HandlerError> {
            Err(HandlerError::new("boom"))
        }
    }

    #[test]
    fn handler_errors_are_logged_in_the_interaction_span() {
        let interaction: Interaction = serde_json::from_value(json!({
            "id": "1100173248714518568",
            "application_id": "1052322265397739523",
            "type": 2,
            "token": "token",
            "version": 1,
            "channel_id": "1052322265397739526",
            "data": { "id": "1052358444704862218", "name": "ping", "type": 1 },
        }))
        .unwrap();

        let lines = lines(|| {
            block_on(dispatch(interaction, &Failing, &())).unwrap_err();
        });

        let line = message(&lines, "interaction failed");
        assert_eq!(line.level, Level::ERROR);
        assert_eq!(line.spans, ["interaction"]);
        assert_eq!(field(line, "name").cloned(), str("ping"));
        assert_eq!(
            field(line, "channel_id").cloned(),
            str("1052322265397739526")
        );
        assert_eq!(field(line, "error").cloned(), str("boom"));
        assert_eq!(field(line, "error_kind").cloned(), str("handler"));
        assert_eq!(field(line, "message").cloned(), str("interaction failed"));
    }
}
