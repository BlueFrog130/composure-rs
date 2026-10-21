---
composure: major
---

# `Platform::bot_token` returns `Option<&str>`

`Platform::bot_token()` now borrows the token (`Option<&str>`) instead of returning a copy, so `ctx.bot()` and `EventCtx::bot()` no longer allocate one per call. Implementations of `Platform` (test stand-ins included) change `fn bot_token(&self) -> Option<String>` to `Option<&str>`.
