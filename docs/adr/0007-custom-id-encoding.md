# Component state is encoded in the custom_id as `prefix:field:field`

Component and Modal Handlers are routed by a `custom_id` prefix and may carry typed state, serialized with a small purpose-built serde format: fields joined by `:` with `\` escaping. Discord caps `custom_id` at 100 characters, so JSON wastes the budget and postcard+base64 adds dependencies and makes ids opaque in logs; the delimiter format is the smallest and stays human-readable. The generated `custom_id(&state)` checks the length at runtime and returns an error rather than letting Discord reject the message.
