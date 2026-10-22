---
composure: minor
---

# REST message bodies from a View

`CreateMessage`, `CreateFollowup`, `EditMessage` and `ExecuteWebhook` implement `TryFrom<&View>`: the components, `IS_COMPONENTS_V2`, the View's other flags (ephemeral only where the Endpoint takes it), its allowed mentions and its files. Building the body fails with a `ViewError`, before anything is sent, when the View cannot be: a `custom_id` that could not be built, or past Discord's limits. `bot.create_message(channel, &CreateMessage::try_from(&view)?)` replaces calling `into_components()` and setting the flag by hand.
