---
composure: patch
---

# Repeated select option values are a `ViewError`

A View whose select repeats an option `value` now fails `View::check` with `ViewError::Duplicate`, naming the value (a `HandlerError` in handlers), instead of Discord dropping the whole response: the user saw "The application did not respond" and the bot logged nothing.
