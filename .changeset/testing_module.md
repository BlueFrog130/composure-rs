---
composure: minor
---

# `composure::testing`: test a bot without Discord

The new `testing` feature (for dev-dependencies) adds `composure::testing`: Interaction builders (`testing::command("echo").option("text", "hi")`, `user_command`, `message_command`, `autocomplete(..).focused(..)`, `component(..).values(..)`, `modal(..).field(..)`, with `.by(user)`, `.in_guild(id)`, `.locale(..)` and more), `TestPlatform<T>` (a `Platform` and `HasData` whose Background Work runs when the test says so), `MockTransport` (records requests, answers from a queue or by route), `TestKey` (signs requests for `adapter::handle_interaction`/`handle_event`), `testing::run`/`answer` (dispatch as an Adapter does) and `ResponseExt` (`text()`, `is_ephemeral()`, `button(label)`, `custom_id(label)`, `embeds()`, `choices()`). The Testing chapter is rewritten around it.
