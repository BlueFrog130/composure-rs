//! Component and Modal Handlers, routed by `custom_id` prefix.

use serde::de::DeserializeOwned;

use crate::types::InteractionResponse;

use super::{ComponentCtx, HandlerError, ModalCtx, custom_id};

/// The `custom_id` prefix a Component Handler answers. Implemented by `#[component]`.
pub trait ComponentDefinition {
    const PREFIX: &'static str;
}

/// Answers clicks and selections on components whose `custom_id` starts with its prefix.
/// Implemented by `#[component]`; listed in `commands!` under `components = [...]`.
#[allow(async_fn_in_trait)] // dispatch is static and Workers futures are not Send
pub trait ComponentHandler<S>: ComponentDefinition {
    /// `fields` is the `custom_id` after `prefix:`, `None` when there is no `:`
    async fn invoke(
        ctx: ComponentCtx<'_, S>,
        fields: Option<&str>,
    ) -> Result<InteractionResponse, HandlerError>;
}

/// The `custom_id` prefix a Modal Handler answers. Implemented by `#[modal]`.
pub trait ModalDefinition {
    const PREFIX: &'static str;
}

/// Answers submits of modals whose `custom_id` starts with its prefix. Implemented by
/// `#[modal]`; listed in `commands!` under `modals = [...]`.
#[allow(async_fn_in_trait)]
pub trait ModalHandler<S>: ModalDefinition {
    async fn invoke(
        ctx: ModalCtx<'_, S>,
        fields: Option<&str>,
    ) -> Result<InteractionResponse, HandlerError>;
}

/// Decodes a handler's `custom_id` state; used by `#[component]` and `#[modal]`
#[doc(hidden)]
pub fn decode_state<T: DeserializeOwned>(
    prefix: &str,
    fields: Option<&str>,
) -> Result<T, HandlerError> {
    custom_id::decode(fields).map_err(|e| HandlerError::new(format!("`{prefix}` {e}")))
}

/// Compile-time check that no two handlers in one list share a prefix. Used by
/// [`commands!`](crate::commands).
#[doc(hidden)]
pub const fn check_prefixes(prefixes: &[&str]) {
    let mut i = 0;
    while i < prefixes.len() {
        let mut j = i + 1;
        while j < prefixes.len() {
            if super::set::str_eq(prefixes[i], prefixes[j]) {
                panic!("two handlers have the same custom_id prefix");
            }
            j += 1;
        }
        i += 1;
    }
}
