# Explicit Command Set over auto-discovery

Commands are registered by listing them in `commands![...]` rather than collected automatically with `inventory`/`linkme`. Distributed-slice and constructor-based registration is unreliable on wasm32 (Cloudflare Workers is a primary target), and an explicit list makes it obvious which Commands a bot serves and which a Sync will publish. The cost is one line per Command; forgetting it means the Command is neither dispatched nor synced, which surfaces immediately.
