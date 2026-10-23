---
composure: patch
---

# Fewer allocations per component and modal

`custom_id` state is written straight into the `custom_id` (one allocation, not one per field) and read in place: a field is copied only when it has escapes, so numbers and ids decode without allocating. A modal's submitted `value` is read by its JSON type instead of through `#[serde(untagged)]`, which buffered each one. In the example bot, a View's button click makes 35 allocations instead of 41, a modal submit 40 instead of 43.

A malformed id now reads "expected an id as a string of digits" instead of "expected SnowflakeVisitor".
