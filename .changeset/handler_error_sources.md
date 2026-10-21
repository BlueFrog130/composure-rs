---
composure: minor
composure_adapter_cloudflare: patch
---

# Handler errors keep their causes

An error converted into a `HandlerError` with `?` keeps its `source()` chain as text: `HandlerError::sources()` lists it, `{:#}` prints `message: cause: cause` (plain `{}` is unchanged), and `HandlerError::with_source(error)` adds causes to an error built by hand. Logged handler errors carry a `sources` field when there are causes, and the Cloudflare adapter without `tracing` logs the whole chain. What the user sees is unchanged.
