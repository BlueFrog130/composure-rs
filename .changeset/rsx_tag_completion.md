---
composure: minor
composure_macros: minor
---

# `rsx!` expands to builders: attributes and placement checked by the compiler, completed by the IDE

As Leptos's `view!` does, `rsx!` markup is now a chain of builder calls: a tag is its type's `builder()` (`Container::builder()`, at the tag), each attribute a documented method (`.accent_color(..)`, `.primary(true)`, `.on_click(..)`), each child a `.child(..)` bounded by a trait per parent (`ContainerChild`, `RowChild`, `SectionChild`, ...), then `.build()`. The builders are public and work by hand too. So:

- an unknown attribute is the compiler's "no method named `accent` found for `ContainerBuilder`", with the closest name; a misplaced child is "`&str` cannot go in a `<Row>`", pointing at it
- rust-analyzer completes and documents attributes as methods, highlights them apart from tags, and completes a tag being typed (`<Con`, also inside another tag) from the built-in tags and the sub-Views in scope; hover and go-to-definition on `<Container>`/`</Container>` reach the type
- the macro's own errors (required attributes, counts, inline handlers) are deduplicated, capped at 16 and each at the offending token; a tag with one is still generated, so it keeps completing while typed
- a missing Prop of a sub-View points at its tag

The markup, its attributes and the messages it builds are unchanged.
