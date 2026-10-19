//! Instrumentation: [`tracing`](https://docs.rs/tracing) spans and events with the `tracing`
//! feature, no-ops without it, so call sites need no `cfg`. As a library, Composure only records;
//! installing a subscriber is the application's (or its Adapter's) job.
//!
//! Spans: `interaction` (every [`dispatch`](crate::command::dispatch)), `background_work`
//! (Deferred Responses), `event` (Webhook Events), `rest` (one per Endpoint call; the route
//! template, never the URL, which can hold an Interaction token).

// Most call sites are behind `rest`
#![allow(unused_macros, dead_code)]

#[cfg(feature = "tracing")]
pub(crate) use tracing::{Instrument, Span};

#[cfg(feature = "tracing")]
macro_rules! debug {
    ($($t:tt)*) => { ::tracing::debug!($($t)*) };
}

#[cfg(feature = "tracing")]
macro_rules! warn {
    ($($t:tt)*) => { ::tracing::warn!($($t)*) };
}

#[cfg(feature = "tracing")]
macro_rules! info_span {
    ($($t:tt)*) => { ::tracing::info_span!($($t)*) };
}

#[cfg(feature = "tracing")]
macro_rules! debug_span {
    ($($t:tt)*) => { ::tracing::debug_span!($($t)*) };
}

#[cfg(not(feature = "tracing"))]
pub(crate) use noop::{Instrument, Span};

#[cfg(not(feature = "tracing"))]
mod noop {
    #[derive(Clone)]
    pub(crate) struct Span;

    impl Span {
        pub(crate) fn in_scope<T>(&self, f: impl FnOnce() -> T) -> T {
            f()
        }
    }

    pub(crate) trait Instrument: Sized {
        fn instrument(self, _span: Span) -> Self {
            self
        }
    }

    impl<T> Instrument for T {}
}

#[cfg(not(feature = "tracing"))]
macro_rules! debug {
    ($($t:tt)*) => {};
}

#[cfg(not(feature = "tracing"))]
macro_rules! warn {
    ($($t:tt)*) => {};
}

#[cfg(not(feature = "tracing"))]
macro_rules! info_span {
    ($($t:tt)*) => {
        $crate::trace::Span
    };
}

#[cfg(not(feature = "tracing"))]
macro_rules! debug_span {
    ($($t:tt)*) => {
        $crate::trace::Span
    };
}
