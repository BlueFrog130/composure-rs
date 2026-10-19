//! The Adapter's own logging: `tracing` events with the `tracing` feature (shown once a
//! subscriber such as [`crate::tracing::init`] is installed), otherwise straight to the console.
//! A literal message plus `name = value` fields (Display): separate indexed fields with
//! `tracing`, appended as `name=value` to the console line without it.

#[cfg(feature = "tracing")]
macro_rules! error {
    ($msg:literal $(, $name:ident = $value:expr)* $(,)?) => {
        ::tracing::error!($($name = %$value,)* $msg)
    };
}

#[cfg(feature = "tracing")]
macro_rules! warn {
    ($msg:literal $(, $name:ident = $value:expr)* $(,)?) => {
        ::tracing::warn!($($name = %$value,)* $msg)
    };
}

#[cfg(not(feature = "tracing"))]
macro_rules! error {
    ($msg:literal $(, $name:ident = $value:expr)* $(,)?) => {
        ::worker::console_error!(
            concat!($msg $(, " ", stringify!($name), "={}")*) $(, $value)*
        )
    };
}

#[cfg(not(feature = "tracing"))]
macro_rules! warn {
    ($msg:literal $(, $name:ident = $value:expr)* $(,)?) => {
        ::worker::console_warn!(
            concat!($msg $(, " ", stringify!($name), "={}")*) $(, $value)*
        )
    };
}

#[cfg(not(feature = "tracing"))]
macro_rules! debug {
    ($msg:literal $(, $name:ident = $value:expr)* $(,)?) => {
        ::worker::console_debug!(
            concat!($msg $(, " ", stringify!($name), "={}")*) $(, $value)*
        )
    };
}
