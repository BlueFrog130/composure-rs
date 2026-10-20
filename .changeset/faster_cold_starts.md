---
default: minor
---

# Faster cold starts on Workers

Workers compile each WebAssembly function on its first call, so an isolate's first request compiled every branch inlined into the functions it ran. Dispatch now polls each interaction kind and each Command, Component, Modal and View handler in a function of its own (`composure::adapter::outlined`, for Adapters too), so the first request compiles only the code it runs, and the Cloudflare adapter does the same for Webhook Events and responses with files. An `#[app]` without a fallback no longer compiles in `Clone` for every Interaction type: 19 KB less wasm.

The Cloudflare adapter's README has a new "Cold starts" section with the build settings the example now uses: `wasm-opt` told not to inline functions that have one caller, and the wasm name section stripped at link time. Measured on workerd against 0.1.0 with worker-build's defaults, the example's startup is about 20% faster, its first request about 30%, warm requests about 25%, and the bundle 29% smaller (21% gzipped). `crates/adapters/cloudflare/bench/tests/coldstart.rs` measures startup, first request and warm latency of any build.
