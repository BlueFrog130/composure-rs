---
composure: minor
composure_macros: minor
---

# Tag and attribute completion and docs in `rsx!`

A hidden `composure::command::tags` module has one module per built-in tag and one documented item per attribute, generated from the macro's own table. `rsx!` refers to it at each attribute's span (and at a half-typed tag's), so rust-analyzer completes tag names and the special attributes (`<Button pri` offers `primary`) and shows their docs on hover. The guide's tag reference is tested against the same table.
