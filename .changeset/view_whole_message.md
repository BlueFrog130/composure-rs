---
composure: major
composure_macros: minor
---

# A View carries the whole message: ephemeral, allowed mentions, files

`View` gains `.ephemeral()`, `.allowed_mentions(..)`, `.suppress_notifications()` and `.file(..)` (with `files()` and `flags()` to read them), and `IntoResponse for View` sends them. `<File>`, `<Thumbnail>` and `<Item>` take `src` (or `url`) as a URL, a `types::File` to upload with the message (its `attachment://` URL is set for you), or an `&Attachment`, through the new `IntoMedia` trait. A placed sub-View's files, flags and mentions go to the View it is placed in. `Update(..)` now drops the ephemeral flag, which an update cannot change, and deferred edits keep the files.

Breaking: `IntoChildren` is gone. What markup can place is now a trait per parent (`ContainerChild`, `RowChild`, ...; see "`rsx!` expands to builders"); a type rendering to a View implements `IntoView` instead, and is placed wherever a View can go.
