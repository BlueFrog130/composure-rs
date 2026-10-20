# REST and deferring

Handlers can call Discord's REST API, and answer later than Discord's 3 seconds allow by
deferring: replying "thinking..." at once and finishing in the background. Both need a State
that implements [`Platform`](https://docs.rs/composure/latest/composure/command/trait.Platform.html), which every Adapter's State does; take
`ctx: Ctx<impl Platform>` to say so and keep the handler testable off any platform.

Enable the `rest` feature (the [`rest`](https://docs.rs/composure/latest/composure/rest/) client and Interaction Endpoints), plus the
`rest-*` features for the [`BotClient`](https://docs.rs/composure/latest/composure/rest/struct.BotClient.html) Endpoints you call, or `tier1`
for the common ones.

## Calling the API

```rust
use composure::{command, command::{Ctx, HandlerError, Platform}};

/// Ask the bot who it is
#[command]
async fn whoami(ctx: Ctx<impl Platform>) -> Result<String, HandlerError> {
    let me = ctx.bot()?.get_current_user().await?.model()?;
    Ok(format!("I'm {}.", me.display_name()))
}
# fn main() {}
```

| Client | Authorized by | For |
|---|---|---|
| [`ctx.client()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.client) → [`InteractionClient`](https://docs.rs/composure/latest/composure/rest/struct.InteractionClient.html) | the Interaction's token (valid 15 minutes) | follow-up messages, the original response: [`followup`](https://docs.rs/composure/latest/composure/rest/struct.InteractionClient.html#method.followup), [`create_followup`](https://docs.rs/composure/latest/composure/rest/struct.InteractionClient.html#method.create_followup), [`edit_original`](https://docs.rs/composure/latest/composure/rest/struct.InteractionClient.html#method.edit_original), [`delete_original`](https://docs.rs/composure/latest/composure/rest/struct.InteractionClient.html#method.delete_original), ... |
| [`ctx.bot()`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.bot) → [`BotClient`](https://docs.rs/composure/latest/composure/rest/struct.BotClient.html) | the bot token (`DISCORD_TOKEN`); an error when the Adapter has none | everything else: messages, members, roles, channels, Commands, ... (one `rest-*` feature per group) |

Every call returns a [`Response<T>`](https://docs.rs/composure/latest/composure/rest/struct.Response.html) whose body is decoded only when you
ask with [`.model()`](https://docs.rs/composure/latest/composure/rest/struct.Response.html#method.model), so calls whose result you ignore skip the
parse. Errors are [`rest::Error`](https://docs.rs/composure/latest/composure/rest/enum.Error.html)s, which convert into
[`HandlerError`](https://docs.rs/composure/latest/composure/command/struct.HandlerError.html) with `?`. Rate limits (429) are waited out, up to
[`with_max_rate_limit_wait`](https://docs.rs/composure/latest/composure/rest/struct.BotClient.html#method.with_max_rate_limit_wait) (2 seconds by
default), then returned as [`Error::RateLimited`](https://docs.rs/composure/latest/composure/rest/enum.Error.html#variant.RateLimited).

Inline calls count toward the 3 seconds. One or two fast calls are fine; anything slower should
defer.

## Deferring

[`ctx.defer(work)`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.defer) answers with a Deferred Response (the user sees
"*bot* is thinking...") and runs `work` as **Background Work** after the response is sent. When
it finishes, its output replaces the "thinking" message, exactly as if the handler had returned
it: text, an embed, a View, a `Result` (an `Err` becomes the error message), files included.

```rust
use std::time::Duration;
use composure::{command, command::{Ctx, Deferred, Ephemeral, HandlerError, Platform}};

/// Count the members of this server (slowly)
#[command]
fn census(ctx: Ctx<impl Platform>) -> Ephemeral<Deferred> {
    Ephemeral(ctx.defer(async |ctx| {
        ctx.sleep(Duration::from_secs(5)).await; // pretend this is slow
        let guild = ctx.guild_id().ok_or_else(|| HandlerError::user("Use this in a server."))?;
        Ok::<_, HandlerError>(format!("Counted the members of {guild}."))
    }))
}
# fn main() {}
```

- The closure receives a [`DeferredCtx`](https://docs.rs/composure/latest/composure/command/struct.DeferredCtx.html): an owned, `'static`
  copy of the Interaction and State, with the same `user()`, `guild_id()`, `client()`, `bot()`,
  `theme()`, plus [`sleep`](https://docs.rs/composure/latest/composure/command/struct.DeferredCtx.html#method.sleep) on the platform's timer.
- Its future must be `Send + 'static`, so it owns what it uses: convert borrowed parameters
  first (`let text = text.to_owned();`) and `move` them in.
- Wrap the result of `defer` in [`Ephemeral`](https://docs.rs/composure/latest/composure/command/struct.Ephemeral.html) to defer ephemerally;
  the final message is then ephemeral too, whatever `work` returns.
- The App's [Theme](theming.md) fills in the result like any response.
- A failing `work` edits the response with the error message and is logged.
- Background Work has a time limit set by the platform: about 30 seconds after the response on
  Cloudflare Workers (`wait_until`), until shutdown's grace period on tokio. The Interaction
  token, which the final edit uses, expires after 15 minutes.

### Updating a component's message later

For components (and modals opened from one),
[`ctx.defer_update(work)`](https://docs.rs/composure/latest/composure/command/struct.Ctx.html#method.defer_update) acknowledges without any visible
loading state and then replaces the message the component is on with `work`'s output:

```rust
use std::time::Duration;
use composure::{component, command::{ComponentCtx, DeferredUpdate, HandlerError, Platform}};

/// Re-roll a die after a dramatic pause
#[component("roll")]
fn reroll(ctx: ComponentCtx<impl Platform>, rolls: u32) -> DeferredUpdate {
    ctx.defer_update(async move |ctx| {
        ctx.sleep(Duration::from_secs(1)).await;
        let face = ctx.interaction().common.id.to_u64() % 6 + 1;
        Ok::<_, HandlerError>(format!("Rolled a {face} ({} rolls)", rolls + 1))
    })
}
# fn main() {}
```

Inline [View](views.md) handlers can do the same, returning new Props from `work`
to re-render the View.

## Follow-ups and progress

Background Work can talk more than once: edit the original response as it goes, and send
follow-up messages (new messages under the original).

```rust
use std::time::Duration;
use composure::{command, command::{Ctx, Deferred, HandlerError, Platform}, rest::EditMessage};

/// Count down, editing the response every second
#[command]
fn countdown(ctx: Ctx<impl Platform>) -> Deferred {
    ctx.defer(async |ctx| {
        let client = ctx.client();
        for n in (1..=3).rev() {
            let content = format!("{n}...");
            client.edit_original(&EditMessage { content: Some(&content), ..Default::default() }).await?;
            ctx.sleep(Duration::from_secs(1)).await;
        }
        client.followup("Liftoff! 🚀").await?;
        Ok::<_, HandlerError>("Done.")
    })
}
# fn main() {}
```

Request bodies ([`EditMessage`](https://docs.rs/composure/latest/composure/rest/struct.EditMessage.html),
[`CreateFollowup`](https://docs.rs/composure/latest/composure/rest/struct.CreateFollowup.html), ...) borrow their fields; absent fields are
left out of the request. [`MessageCallbackData`](https://docs.rs/composure/latest/composure/types/struct.MessageCallbackData.html) converts into
both. REST bodies are never [themed](theming.md); build them with `ctx.theme()`'s
helpers to match.

## Outside Interactions

A [`BotClient`](https://docs.rs/composure/latest/composure/rest/struct.BotClient.html) can also be built from any
[`Transport`](https://docs.rs/composure/latest/composure/rest/trait.Transport.html), a bot token and the application id, for cron jobs,
queues or scripts: `BotClient::new(transport, &token, application_id)`, or
[`Credentials::bot_client(transport)`](https://docs.rs/composure/latest/composure/credentials/struct.Credentials.html#method.bot_client). The Adapters provide one
(`bot_client(&env)` on Workers, `host.bot()` on tokio), and the `reqwest` feature adds a blocking
[`ReqwestTransport`](https://docs.rs/composure/latest/composure/rest/struct.ReqwestTransport.html) for deploy-time tools. See
[Platforms](platforms.md).

Webhook Event handlers get the bot client from their
[`EventCtx`](https://docs.rs/composure/latest/composure/command/struct.EventCtx.html#method.bot); see [Webhook Events](events.md).
