use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct TypeField<const T: u8>;

impl<const T: u8> Serialize for TypeField<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(T)
    }
}

impl<'de, const T: u8> Deserialize<'de> for TypeField<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        if value == T {
            Ok(TypeField::<T>)
        } else {
            Err(serde::de::Error::custom("not an option type"))
        }
    }
}
