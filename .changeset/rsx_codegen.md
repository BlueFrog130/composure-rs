---
composure_macros: patch
---

# Leaner `rsx!` code

Adjacent string literals are joined at compile time, and each markup level reserves room for the children it shows (a `<Text>` for its literals and expressions), so a level allocates once. Output is unchanged; the example bot's card View allocates once less.
