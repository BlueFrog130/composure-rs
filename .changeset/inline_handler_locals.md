---
composure_macros: minor
---

# Inline handlers that use a local are a clear error

An inline handler is hoisted out of the `#[view]` function, so it only sees Props. Using a local of the function body (`let total = ..`), or a parameter of a closure around nested markup (`{items.iter().map(|item| rsx! { .. })}`), is now an error at the use ("`total` is not a Prop: inline handlers only see Props ..."), and so is a local that shadows a Prop. Before, rustc reported "cannot find value" in code the user never wrote.
