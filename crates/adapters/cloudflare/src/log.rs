//! The Adapter's own logging: `tracing` events with the `tracing` feature (shown once a
//! subscriber such as [`crate::tracing::init`] is installed), otherwise straight to the console.
//! Messages are format strings so both work.

#[cfg(feature = "tracing")]
macro_rules! error {
    ($($t:tt)*) => { ::tracing::error!($($t)*) };
}

#[cfg(feature = "tracing")]
macro_rules! warn {
    ($($t:tt)*) => { ::tracing::warn!($($t)*) };
}

#[cfg(feature = "tracing")]
macro_rules! debug {
    ($($t:tt)*) => { ::tracing::debug!($($t)*) };
}

#[cfg(not(feature = "tracing"))]
macro_rules! error {
    ($($t:tt)*) => { ::worker::console_error!($($t)*) };
}

#[cfg(not(feature = "tracing"))]
macro_rules! warn {
    ($($t:tt)*) => { ::worker::console_warn!($($t)*) };
}

#[cfg(not(feature = "tracing"))]
macro_rules! debug {
    ($($t:tt)*) => { ::worker::console_debug!($($t)*) };
}
