---
default: patch
---

# Deferred update errors become ephemeral follow-ups; Components V2 conflicts are caught

When `ctx.defer_update(..)` work fails, the error is now sent as an ephemeral follow-up instead of replacing the component's message, which kept failing on Components V2 messages and lost the message's state (#12).

A Components V2 response or deferred edit that also sets `content`, `embeds` or a poll is now a `HandlerError` naming the conflicting field, instead of Discord's bare "This interaction failed" (#13).
