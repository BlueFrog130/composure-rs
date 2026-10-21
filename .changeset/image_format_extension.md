---
composure: major
---

# `ImageFormat::extension` replaces `AsRef<str>`; `strum` is gone

`ImageFormat` no longer implements `AsRef<str>`, which returned the variant name (`"Png"`), not a file extension. `ImageFormat::extension()` returns `png`, `jpg`, `webp` or `gif`, and `ImageFormat` is now `Clone` and `Copy`. `composure` no longer depends on `strum`.
