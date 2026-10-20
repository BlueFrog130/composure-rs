//! [`outlined`], re-exported from [`adapter`](crate::adapter) for Adapters; dispatch uses it too.

use std::{
    future::{Future, poll_fn},
    pin::{Pin, pin},
    task::{Context, Poll},
};

/// Awaits `future` with its `poll` in a function of its own, which the optimizer cannot merge
/// into the caller's.
///
/// On WebAssembly hosts that compile each function on its first call (V8, so Workers), a branch
/// inlined into a hot function is compiled whenever that function first runs, taken or not.
/// Outlining the rarely taken or handler-specific branches keeps an isolate's first request from
/// compiling code it doesn't run. Costs one call per poll.
pub async fn outlined<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    poll_fn(|cx| poll_outlined(future.as_mut(), cx)).await
}

#[inline(never)]
fn poll_outlined<F: Future>(future: Pin<&mut F>, cx: &mut Context<'_>) -> Poll<F::Output> {
    future.poll(cx)
}
