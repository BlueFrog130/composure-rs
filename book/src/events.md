# Webhook Events

[Webhook Events](https://discord.com/developers/docs/events/webhook-events) are the few
gateway-style events Discord delivers over HTTP: an app being installed or removed,
entitlements being created or changed, and lobby messages. Composure receives them on the same
server as Interactions and hands them to an [`EventHandler`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html).

```rust
use composure::{
    app,
    command::{EventCtx, EventHandler, HandlerError, Platform},
    types::{ApplicationAuthorized, ApplicationIntegrationType},
};

/// Welcomes users who install the app to their account
pub struct Events;

impl<S: Platform> EventHandler<S> for Events {
    async fn application_authorized(
        &self,
        ctx: EventCtx<'_, S>,
        event: ApplicationAuthorized,
    ) -> Result<(), HandlerError> {
        if event.integration_type != Some(ApplicationIntegrationType::UserInstall) {
            return Ok(());
        }
        let bot = ctx.bot()?;
        let dm = bot.create_dm(event.user.id).await?.model()?;
        bot.send_message(dm.id, "Thanks for installing! Try `/hello` anywhere.").await?;
        Ok(())
    }
}

/// Say hello
#[composure::command]
fn hello() -> &'static str {
    "Hello!"
}

#[app(handlers(hello), events = Events)]
pub struct Bot;
# fn main() {}
```

## Setting it up

1. Implement [`EventHandler<S>`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html) on a unit struct, overriding the
   methods for the events you want; the rest ignore their event.
2. Register it on an App with `#[app(.., events = Events)]`. At most one App in a tree sets
   `events` (a bot has one events URL); the root App forwards to it.
3. In the [Developer Portal](https://discord.com/developers/applications), under *Webhooks*, set
   the Events URL to `https://<your bot>/events` and subscribe to the event types.

The Adapter verifies the request signature like an Interaction's, answers Discord with `204`
immediately, and only then runs the handler as Background Work. So handlers can take their time
(within the platform's Background Work limit), but their errors are only logged: Discord does
not retry.

## Events

| Method | Event | Payload |
|---|---|---|
| [`application_authorized`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html#method.application_authorized) | The app was installed to a server or a user | [`ApplicationAuthorized`](https://docs.rs/composure/latest/composure/types/struct.ApplicationAuthorized.html) |
| [`application_deauthorized`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html#method.application_deauthorized) | A user removed the app | [`ApplicationDeauthorized`](https://docs.rs/composure/latest/composure/types/struct.ApplicationDeauthorized.html) |
| [`entitlement_create`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html#method.entitlement_create) | A user or guild bought a SKU | [`Entitlement`](https://docs.rs/composure/latest/composure/types/struct.Entitlement.html) |
| [`entitlement_update`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html#method.entitlement_update) | A subscription renewed or ended | [`Entitlement`](https://docs.rs/composure/latest/composure/types/struct.Entitlement.html) |
| [`entitlement_delete`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html#method.entitlement_delete) | An entitlement was refunded or removed | [`Entitlement`](https://docs.rs/composure/latest/composure/types/struct.Entitlement.html) |
| [`lobby_message_create`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html#method.lobby_message_create) | A message was sent in a lobby | [`LobbyMessage`](https://docs.rs/composure/latest/composure/types/struct.LobbyMessage.html) |
| [`lobby_message_update`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html#method.lobby_message_update) | A lobby message was edited | [`LobbyMessage`](https://docs.rs/composure/latest/composure/types/struct.LobbyMessage.html) |
| [`lobby_message_delete`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html#method.lobby_message_delete) | A lobby message was deleted | [`LobbyMessageDelete`](https://docs.rs/composure/latest/composure/types/struct.LobbyMessageDelete.html) |
| [`unknown`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html#method.unknown) | Any other type (game DMs, quests, types added later), or one whose data did not match its model | the type name and raw JSON |

Override [`handle`](https://docs.rs/composure/latest/composure/command/trait.EventHandler.html#method.handle) instead to see every
[`Event`](https://docs.rs/composure/latest/composure/types/enum.Event.html) before it is routed to a method.

An unknown or malformed event never fails the request: it arrives in `unknown`, so a bot keeps
working when Discord adds event types.

## The context

[`EventCtx`](https://docs.rs/composure/latest/composure/command/struct.EventCtx.html) carries the Adapter's
[`state()`](https://docs.rs/composure/latest/composure/command/struct.EventCtx.html#method.state), the
[`application_id()`](https://docs.rs/composure/latest/composure/command/struct.EventCtx.html#method.application_id), the event's
[`timestamp()`](https://docs.rs/composure/latest/composure/command/struct.EventCtx.html#method.timestamp) (events can arrive late and out of order),
and, with a [`Platform`](https://docs.rs/composure/latest/composure/command/trait.Platform.html) State, [`bot()`](https://docs.rs/composure/latest/composure/command/struct.EventCtx.html#method.bot)
for REST. There is no Interaction, so no Interaction client or Theme.

With the `tracing` feature, handlers run inside an `event` span carrying `event_type`.
