//! Autocomplete Handlers: suggestions for an Option while the user types.

use std::fmt::Display;

use crate::types::{
    ApplicationCommandOptionChoice, ApplicationCommandOptionChoiceValue, InteractionResponse,
};

use super::{Ctx, HandlerError, response::into_handler_error};

/// Discord shows at most 25 suggestions; extra ones are dropped
pub const MAX_CHOICES: usize = 25;

/// Suggests Choices for an Option. Implemented by `#[autocomplete]`; attached to an Option with
/// `#[option("...", autocomplete = handler)]`.
pub trait Autocomplete<S> {
    /// `input` is what the user has typed so far
    fn complete(
        ctx: Ctx<'_, S>,
        input: &str,
    ) -> impl Future<Output = Result<InteractionResponse, HandlerError>> + Send;
}

/// What an Autocomplete Handler may return: a `Vec` of suggestions, or a `Result` of one
pub trait AutocompleteOutput {
    /// The autocomplete Response, or the error to answer with instead
    fn into_result(self) -> Result<InteractionResponse, HandlerError>;
}

impl<T: IntoChoice> AutocompleteOutput for Vec<T> {
    fn into_result(self) -> Result<InteractionResponse, HandlerError> {
        Ok(InteractionResponse::respond_with_autocomplete_choices(
            self.into_iter()
                .take(MAX_CHOICES)
                .map(IntoChoice::into_choice)
                .collect(),
        ))
    }
}

impl<T: AutocompleteOutput, E: Display + 'static> AutocompleteOutput for Result<T, E> {
    fn into_result(self) -> Result<InteractionResponse, HandlerError> {
        self.map_err(into_handler_error)?.into_result()
    }
}

/// One suggestion: a string (shown and sent as is), a number, `(name, value)`, or a full
/// [`ApplicationCommandOptionChoice`]
pub trait IntoChoice {
    /// The suggestion's name (shown) and value (sent)
    fn into_choice(self) -> ApplicationCommandOptionChoice;
}

impl IntoChoice for ApplicationCommandOptionChoice {
    fn into_choice(self) -> ApplicationCommandOptionChoice {
        self
    }
}

impl IntoChoice for String {
    fn into_choice(self) -> ApplicationCommandOptionChoice {
        (self.clone(), self).into_choice()
    }
}

impl IntoChoice for &str {
    fn into_choice(self) -> ApplicationCommandOptionChoice {
        (self, self).into_choice()
    }
}

macro_rules! number_choice {
    ($($ty:ty),*) => {$(
        impl IntoChoice for $ty {
            fn into_choice(self) -> ApplicationCommandOptionChoice {
                (self.to_string(), self).into_choice()
            }
        }
    )*};
}

number_choice!(i8, i16, i32, i64, u8, u16, u32, f64);

impl<N: Into<String>, V: ChoiceValue> IntoChoice for (N, V) {
    fn into_choice(self) -> ApplicationCommandOptionChoice {
        ApplicationCommandOptionChoice {
            name: self.0.into(),
            name_localizations: None,
            value: self.1.into_value(),
        }
    }
}

/// The value half of a `(name, value)` suggestion: a string for string Options, an integer or
/// number for numeric ones
pub trait ChoiceValue {
    /// The value, typed for its Option
    fn into_value(self) -> ApplicationCommandOptionChoiceValue;
}

impl ChoiceValue for String {
    fn into_value(self) -> ApplicationCommandOptionChoiceValue {
        ApplicationCommandOptionChoiceValue::String(self)
    }
}

impl ChoiceValue for &str {
    fn into_value(self) -> ApplicationCommandOptionChoiceValue {
        ApplicationCommandOptionChoiceValue::String(self.to_string())
    }
}

macro_rules! integer_value {
    ($($ty:ty),*) => {$(
        impl ChoiceValue for $ty {
            fn into_value(self) -> ApplicationCommandOptionChoiceValue {
                ApplicationCommandOptionChoiceValue::Integer(self.into())
            }
        }
    )*};
}

integer_value!(i8, i16, i32, i64, u8, u16, u32);

impl ChoiceValue for f64 {
    fn into_value(self) -> ApplicationCommandOptionChoiceValue {
        ApplicationCommandOptionChoiceValue::Double(self)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn choices(output: impl AutocompleteOutput) -> serde_json::Value {
        serde_json::to_value(output.into_result().unwrap()).unwrap()["data"]["choices"].clone()
    }

    #[test]
    fn suggestions_become_choices() {
        assert_eq!(
            choices(vec!["apple", "pear"]),
            json!([{ "name": "apple", "value": "apple" }, { "name": "pear", "value": "pear" }])
        );
        assert_eq!(
            choices(vec![("One", 1i64), ("Two", 2)]),
            json!([{ "name": "One", "value": 1 }, { "name": "Two", "value": 2 }])
        );
        assert_eq!(
            choices(vec![2.5f64]),
            json!([{ "name": "2.5", "value": 2.5 }])
        );
        assert_eq!(
            choices((0..40u32).collect::<Vec<_>>())
                .as_array()
                .unwrap()
                .len(),
            25
        );
    }

    #[test]
    fn errors_are_handler_errors() {
        let output: Result<Vec<String>, String> = Err("down".into());
        assert_eq!(output.into_result().unwrap_err().to_string(), "down");
        let output: Result<Vec<String>, HandlerError> = Err(HandlerError::user("nope"));
        assert_eq!(
            output.into_result().unwrap_err().kind(),
            super::super::ErrorKind::User
        );
    }
}
