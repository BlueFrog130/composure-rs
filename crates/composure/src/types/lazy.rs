use std::{cell::OnceCell, fmt};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};
use serde_json::value::RawValue;

/// A field kept as received JSON and parsed on first read, so an Interaction pays for it only
/// when a handler looks at it. Used for big objects most handlers ignore, such as the message a
/// component is attached to.
pub struct Lazy<T> {
    raw: Box<RawValue>,
    /// Boxed so an unread Lazy stays small whatever `T` is
    parsed: OnceCell<Option<Box<T>>>,
}

impl<T: DeserializeOwned> Lazy<T> {
    /// The parsed value, or `None` if the JSON does not match `T`. [`Lazy::parse`] gives the error.
    pub fn get(&self) -> Option<&T> {
        self.parsed
            .get_or_init(|| self.parse().ok().map(Box::new))
            .as_deref()
    }

    /// Parses the JSON again, returning why it does not match `T`
    pub fn parse(&self) -> serde_json::Result<T> {
        serde_json::from_slice(self.raw.get().as_bytes())
    }
}

impl<T: Serialize> Lazy<T> {
    /// A Lazy holding `value` already parsed
    pub fn new(value: T) -> serde_json::Result<Self> {
        Ok(Self {
            raw: serde_json::value::to_raw_value(&value)?,
            parsed: OnceCell::from(Some(Box::new(value))),
        })
    }
}

impl<T> Lazy<T> {
    /// The JSON as received
    pub fn raw(&self) -> &RawValue {
        &self.raw
    }
}

/// Clones the JSON; the clone parses again when read
impl<T> Clone for Lazy<T> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw.clone(),
            parsed: OnceCell::new(),
        }
    }
}

impl<T> fmt::Debug for Lazy<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Lazy").field(&self.raw.get()).finish()
    }
}

impl<'de, T> Deserialize<'de> for Lazy<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self {
            raw: Box::<RawValue>::deserialize(deserializer)?,
            parsed: OnceCell::new(),
        })
    }
}

impl<T> Serialize for Lazy<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.raw.serialize(serializer)
    }
}
