---
composure_macros: patch
---

# Leaner `rsx!` code

Adjacent string literals are joined at compile time: a text of literals only becomes one `String::from`, a mixed one starts with room for its literals and expressions. A markup level without `{..}` children or sub-Views is built as `vec![..]` directly, and one error slot per `rsx!` is shared by every level instead of one per level. Output is unchanged; the example bot's card View allocates once less.
