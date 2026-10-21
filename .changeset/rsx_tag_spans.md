---
composure_macros: minor
---

# Built-in `rsx!` tags keep their span

`rsx!` expands each built-in tag to its type at the tag name's span, the opening and the closing tag alike (aliases such as `<Row>` name the real type, `ActionRow`). Hover, go-to-definition and semantic highlighting now work on `<Container>` and `</Container>`.
