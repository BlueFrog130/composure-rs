---
composure_macros: minor
---

# `rsx!` reports every markup error and keeps expanding

`rsx!` parses markup with rstml's recoverable parser and reports every error at once instead of stopping at the first: misplaced tags, bad attributes, unclosed or mismatched tags. A tag with an error is left out and the rest of the markup still expands (to `View::default()` or a placeholder `ModalView` when nothing parsed), so `{..}` blocks keep type-checking and rust-analyzer keeps completing them while a tag is half-typed. A `#[view]` whose markup has errors still gets its Props struct, builder and `ViewHandler`, so code using the View no longer reports errors of its own.
