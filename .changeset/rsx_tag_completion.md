---
composure: minor
composure_macros: minor
---

# Tag and attribute completion and docs in `rsx!`

A hidden `composure::command::tags` module has one module per built-in tag and one documented item per attribute, generated from the macro's own table. `rsx!` refers to it at each attribute's span, so rust-analyzer shows attribute docs on hover, even on a tag that does not compile yet. While rust-analyzer completes, the cursor's position in the raw markup decides what to offer: after `<` the tags (`<Con` offers `Container`), in an open tag that tag's attributes (`<Button pri` offers `primary`). Ordinary builds get no extra references or errors. The guide's tag reference is tested against the same table.
