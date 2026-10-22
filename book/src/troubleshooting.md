# Troubleshooting

Discord shows "This interaction failed" (or "The application did not respond") when it gets no
valid answer in time. Composure answers handler errors itself, with an ephemeral message the
user reads and a log line with the cause, so Discord's own failure usually means the answer
never arrived or Discord rejected it. Start with the logs: `wrangler tail` or Workers Logs on
Cloudflare, stderr on a server.

## "This interaction failed"

| Cause | What you see | Fix |
|---|---|---|
| No answer within 3 seconds | "The application did not respond"; the handler finishes later | Answer with `ctx.defer(..)` and do the slow part in Background Work ([REST and deferring](deferring.md)). On Workers, a cold start plus slow REST calls before answering can be enough |
| Signature or public key mismatch | 401 in the logs; the Endpoint URL won't save | Set `DISCORD_PUBLIC_KEY` to the public key of the application whose Endpoint URL points at the bot, not another one's (development and production apps differ) |
| Configuration failed to load | 500 on Workers, logged on the isolate's first request | A missing secret or var, or an error from `.with_credentials` / `.with_data`; set it and retry |
| Discord rejected the response | The handler ran, nothing is logged as an error | A response Discord refuses: a component over its limits (5 buttons per row, 25 select options, label lengths), more than 10 embeds, an empty message. Compare the response with Discord's limits; [Testing](testing.md) shows the JSON |
| Components V2 with content or embeds | An error naming the field | A Components V2 message (a `Container`, a View) carries its text in text displays: move `content` and `embeds` into components |

## Errors the user sees

| Message | Cause | Fix |
|---|---|---|
| "Unknown command." | The App has no Command with that name and type: Discord's Commands are out of sync with the deployed code | `cargo run --bin sync -- --check` shows the differences; sync, or deploy the code first ([Deploying](deploying.md)) |
| "Something went wrong handling this interaction." | A handler returned an error, or an Option couldn't be read | The log line has the error and its causes (`sources`) |
| The same, from a button posted before a deploy | Its `custom_id` was encoded by an older version of the handler: the state's fields changed | Keep the state's fields compatible, or give the handler a new prefix so old buttons get the "not handled" answer instead |
| The same, logged as "custom_id is N characters" | The state doesn't fit Discord's 100 characters | Keep less in the `custom_id`: an id, with the rest in storage |
| "This bot does not handle components." | No handler for the `custom_id`'s prefix | List the handler (or the View) in the App, or set a fallback ([Components](components.md#fallback)) |

## REST errors

`ctx.bot()` and `ctx.client()` calls return [`rest::Error`](https://docs.rs/composure/latest/composure/rest/enum.Error.html),
with Discord's JSON error code when it sent one.

| Status | Cause | Fix |
|---|---|---|
| 401 from `ctx.bot()` | `DISCORD_TOKEN` is missing, wrong, or reset in the Developer Portal | Set the current bot token |
| 401 from `ctx.client()` | The Interaction's token expired: follow-ups and edits work for 15 minutes | Finish within 15 minutes, or post with `ctx.bot()` |
| 403, code 50013 (Missing Permissions) | The bot lacks a permission in that channel, or the target's role is above the bot's | Grant the permission (the invite link's `permissions`), or move the bot's role up |
| 403, code 50001 (Missing Access) | The bot can't see the channel or isn't in the server, as with a user-installed app used in a server without the bot | Check `ctx.interaction().common.authorizing_integration_owners` before calling the bot client, or answer through `ctx.client()`, which needs no membership |
| 429 | Rate limited for longer than the client waits (2 seconds by default) | `with_max_rate_limit_wait(..)` on the client, or do less per Interaction |

## Other surprises

- **A Command doesn't show up.** Sync it; global Commands can take a moment to appear in the
  client, and a guild-only Command shows only where it was sent. Restarting Discord (Ctrl+R)
  refreshes its cache.
- **An ephemeral deferral turned the answer ephemeral.** Ephemerality is decided by the first
  response: `Ephemeral(ctx.defer(..))` makes the final message ephemeral too.
- **Webhook Events don't arrive.** The Webhook Events URL must end in `/events`, and each event
  must be enabled in the Developer Portal.
