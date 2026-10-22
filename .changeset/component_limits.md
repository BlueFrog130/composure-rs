---
composure: minor
composure_macros: minor
---

# Discord's component limits are checked

A View or `ModalView` answered with (or sent through REST) is checked against Discord's limits in one walk, without allocating: 40 components, 4000 characters of text, 5 buttons or one select per row, 25 select options, 2-10 radio and 1-10 checkbox options, 1-3 texts per section, 10 gallery items, 100-character `custom_id`s, 80-character button labels, and a modal's 5 top-level components and 45-character title. A violation is a `ViewError::Limit` naming the limit and the count (a `HandlerError` in handlers) instead of Discord's bare "This interaction failed". The limits are `types::limits`; `View::check` and `ModalView::check` run the walk. `rsx!` reports the counts it can see at compile time: more than 5 buttons in a `<Row>`, a select beside anything else, more than 3 texts in a `<Section>`, more than 5 children in a `<Modal>`, more than 25 `<Option>`s (10 in a group) or 10 `<Item>`s.
