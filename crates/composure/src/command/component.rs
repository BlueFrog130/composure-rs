//! Component and Modal Handlers, routed by `custom_id` prefix, and Modals as typed forms.

use serde::de::DeserializeOwned;

use crate::types::{
    ActionRow, Component, InteractionResponse, MessageComponentInteraction, ModalCallbackData,
    ModalSubmitData, ModalSubmitInteraction, TextInputStyle,
};

use super::{HandlerError, custom_id};

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
        interaction: &MessageComponentInteraction,
        state: &S,
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
        interaction: &ModalSubmitInteraction,
        state: &S,
        fields: Option<&str>,
    ) -> Result<InteractionResponse, HandlerError>;
}

/// A form shown as a modal, one text input per field. Derived with `#[derive(Modal)]`.
pub trait Modal: Sized {
    /// Shown at the top of the modal (max 45 characters)
    const TITLE: &'static str;

    /// One action row per field, each holding its text input
    fn components() -> Vec<ActionRow>;

    /// The submitted fields
    fn extract(data: &ModalSubmitData) -> Result<Self, HandlerError>;

    /// This form as a modal response with `custom_id`
    fn response(custom_id: String) -> InteractionResponse {
        InteractionResponse::Modal(ModalCallbackData {
            custom_id,
            title: Self::TITLE.to_string(),
            components: Some(Self::components()),
            tts: None,
            content: None,
            embeds: None,
            allowed_mentions: None,
            flags: None,
        })
    }
}

/// A Modal field's type: `String` (required) or `Option<String>`
pub trait ModalField: Sized {
    const REQUIRED: bool;

    fn extract(value: Option<&str>) -> Result<Self, String>;
}

impl ModalField for String {
    const REQUIRED: bool = true;

    fn extract(value: Option<&str>) -> Result<Self, String> {
        value
            .map(str::to_string)
            .ok_or_else(|| "missing".to_string())
    }
}

impl ModalField for Option<String> {
    const REQUIRED: bool = false;

    fn extract(value: Option<&str>) -> Result<Self, String> {
        Ok(value.filter(|v| !v.is_empty()).map(str::to_string))
    }
}

/// Settings of one Modal field, from its `#[input]` attribute
#[derive(Debug, Clone, Copy)]
pub struct InputMeta {
    pub custom_id: &'static str,
    pub label: &'static str,
    pub paragraph: bool,
    pub min_length: Option<i32>,
    pub max_length: Option<i32>,
    pub placeholder: Option<&'static str>,
    pub value: Option<&'static str>,
}

impl InputMeta {
    /// The field's text input in its own action row
    pub fn row(&self, required: bool) -> ActionRow {
        let style = if self.paragraph {
            TextInputStyle::Paragraph
        } else {
            TextInputStyle::Short
        };
        ActionRow::new(vec![Component::new_text_input(
            self.custom_id.to_string(),
            style,
            self.label.to_string(),
            self.min_length,
            self.max_length,
            Some(required),
            self.value.map(str::to_string),
            self.placeholder.map(str::to_string),
        )])
    }

    pub fn extract<T: ModalField>(&self, data: &ModalSubmitData) -> Result<T, HandlerError> {
        T::extract(data.value(self.custom_id))
            .map_err(|e| HandlerError::invalid_option(self.custom_id, e))
    }
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
