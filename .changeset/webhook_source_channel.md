---
default: minor
---

# Channel follower webhooks have their source

`types::Webhook` now has `source_guild` and `source_channel` (new `WebhookSourceGuild` and `WebhookSourceChannel`), which Discord sends for channel follower webhooks: which announcement channel a server follows can be read from Get Guild Webhooks.
