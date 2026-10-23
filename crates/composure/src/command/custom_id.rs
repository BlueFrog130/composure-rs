//! State carried in a component's or modal's `custom_id` (ADR 0007): `prefix:field:field`, each
//! field `\`-escaped. Values are written field by field in declaration order (structs and tuples
//! flatten); the format is not self-describing, so decoding needs the same type.
//!
//! - integers, floats, `bool` (`1`/`0`), `char`, strings, [`Snowflake`](crate::types::Snowflake)
//! - `Option`: an empty field is `None`, so `Some(String::new())` decodes as `None`
//! - enums: the variant index, then its fields
//! - sequences and maps: the length, then the items

use std::{
    borrow::Cow,
    fmt::{self, Display, Write},
};

use serde::{
    Serialize,
    de::{self, DeserializeOwned, IntoDeserializer, Visitor},
    ser,
};

/// Discord's `custom_id` limit
pub const MAX_LEN: usize = 100;

const SEPARATOR: char = ':';
const ESCAPE: char = '\\';

/// A `custom_id` that could not be built or read
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomIdError {
    /// Longer than Discord's 100 characters
    TooLong(usize),
    /// The state has a shape the format cannot carry, or did not match the expected type
    Invalid(String),
}

impl Display for CustomIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CustomIdError::TooLong(len) => {
                write!(f, "custom_id is {len} characters, Discord allows {MAX_LEN}")
            }
            CustomIdError::Invalid(message) => write!(f, "custom_id: {message}"),
        }
    }
}

impl std::error::Error for CustomIdError {}

impl ser::Error for CustomIdError {
    fn custom<T: Display>(msg: T) -> Self {
        CustomIdError::Invalid(msg.to_string())
    }
}

impl de::Error for CustomIdError {
    fn custom<T: Display>(msg: T) -> Self {
        CustomIdError::Invalid(msg.to_string())
    }
}

type Result<T, E = CustomIdError> = std::result::Result<T, E>;

/// `prefix` followed by `state`'s fields; an error past 100 characters
pub fn encode<T: Serialize + ?Sized>(prefix: &str, state: &T) -> Result<String> {
    // One allocation: state rarely takes a `custom_id` past Discord's limit
    let mut out = String::with_capacity(MAX_LEN);
    out.push_str(prefix);
    let mut writer = Writer { out };
    state.serialize(&mut writer)?;
    // Characters, not bytes, but never more characters than bytes
    if writer.out.len() > MAX_LEN {
        let len = writer.out.chars().count();
        if len > MAX_LEN {
            return Err(CustomIdError::TooLong(len));
        }
    }
    Ok(writer.out)
}

/// The prefix and the encoded fields after it (`None` when there are none)
pub fn split(custom_id: &str) -> (&str, Option<&str>) {
    let mut escaped = false;
    for (i, c) in custom_id.char_indices() {
        match c {
            _ if escaped => escaped = false,
            ESCAPE => escaped = true,
            SEPARATOR => return (&custom_id[..i], Some(&custom_id[i + 1..])),
            _ => {}
        }
    }
    (custom_id, None)
}

/// Decodes the fields after the prefix, as given by [`split`]
pub fn decode<T: DeserializeOwned>(fields: Option<&str>) -> Result<T> {
    let mut reader = Reader { rest: fields };
    let value = T::deserialize(&mut reader)?;
    match reader.remaining() {
        0 => Ok(value),
        n => Err(CustomIdError::Invalid(format!(
            "{n} unexpected trailing fields"
        ))),
    }
}

/// `field` without its escapes; borrowed unless it has any
fn unescape(field: &str) -> Cow<'_, str> {
    if !field.contains(ESCAPE) {
        return Cow::Borrowed(field);
    }
    let mut out = String::with_capacity(field.len());
    let mut escaped = false;
    for c in field.chars() {
        match c {
            ESCAPE if !escaped => escaped = true,
            _ => {
                escaped = false;
                out.push(c);
            }
        }
    }
    Cow::Owned(out)
}

struct Writer {
    out: String,
}

impl Writer {
    fn field(&mut self, value: impl Display) {
        self.out.push(SEPARATOR);
        // Formatted straight into the `custom_id`, escaping as it goes
        let _ = write!(Escaping(&mut self.out), "{value}");
    }

    fn unsupported<T>(what: &str) -> Result<T> {
        Err(CustomIdError::Invalid(format!("{what} cannot be encoded")))
    }
}

impl ser::Serializer for &mut Writer {
    type Ok = ();
    type Error = CustomIdError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.field(u8::from(v));
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_char(self, v: char) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_str(self, v: &str) -> Result<()> {
        self.field(v);
        Ok(())
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<()> {
        Writer::unsupported("bytes")
    }
    fn serialize_none(self) -> Result<()> {
        self.field("");
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<()> {
        Ok(())
    }
    fn serialize_unit_variant(self, _: &'static str, index: u32, _: &'static str) -> Result<()> {
        self.field(index);
        Ok(())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        index: u32,
        _: &'static str,
        value: &T,
    ) -> Result<()> {
        self.field(index);
        value.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self> {
        let len = len.ok_or_else(|| CustomIdError::Invalid("unsized sequence".into()))?;
        self.field(len);
        Ok(self)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        index: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self> {
        self.field(index);
        Ok(self)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self> {
        let len = len.ok_or_else(|| CustomIdError::Invalid("unsized map".into()))?;
        self.field(len);
        Ok(self)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        index: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self> {
        self.field(index);
        Ok(self)
    }
}

/// Appends to a `custom_id` with separators and escapes escaped
struct Escaping<'a>(&'a mut String);

impl Write for Escaping<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if c == SEPARATOR || c == ESCAPE {
                self.0.push(ESCAPE);
            }
            self.0.push(c);
        }
        Ok(())
    }
}

macro_rules! compound {
    ($($trait:ident :: $method:ident),*) => {$(
        impl ser::$trait for &mut Writer {
            type Ok = ();
            type Error = CustomIdError;
            fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
                value.serialize(&mut **self)
            }
            fn end(self) -> Result<()> {
                Ok(())
            }
        }
    )*};
}

compound!(
    SerializeSeq::serialize_element,
    SerializeTuple::serialize_element,
    SerializeTupleStruct::serialize_field,
    SerializeTupleVariant::serialize_field
);

impl ser::SerializeMap for &mut Writer {
    type Ok = ();
    type Error = CustomIdError;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<()> {
        key.serialize(&mut **self)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeStruct for &mut Writer {
    type Ok = ();
    type Error = CustomIdError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, _: &'static str, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut Writer {
    type Ok = ();
    type Error = CustomIdError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, _: &'static str, value: &T) -> Result<()> {
        value.serialize(&mut **self)
    }
    fn end(self) -> Result<()> {
        Ok(())
    }
}

/// Reads fields off the `custom_id` in place: only a field with escapes is copied
struct Reader<'a> {
    /// The fields not read yet, still escaped; `None` once every field is read
    rest: Option<&'a str>,
}

impl<'a> Reader<'a> {
    fn next(&mut self) -> Result<Cow<'a, str>> {
        let rest = self
            .rest
            .ok_or_else(|| CustomIdError::Invalid("too few fields".into()))?;
        let (field, rest) = split(rest);
        self.rest = rest;
        Ok(unescape(field))
    }

    /// Whether the next field is empty (`None` for an `Option`)
    fn next_is_empty(&self) -> bool {
        self.rest
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(SEPARATOR))
    }

    /// How many fields are left
    fn remaining(&self) -> usize {
        std::iter::successors(self.rest, |rest| split(rest).1).count()
    }

    fn parse<T: std::str::FromStr>(&mut self, what: &str) -> Result<T> {
        let field = self.next()?;
        field
            .parse()
            .map_err(|_| CustomIdError::Invalid(format!("`{field}` is not a valid {what}")))
    }

    fn len(&mut self) -> Result<usize> {
        self.parse("length")
    }
}

macro_rules! parse_number {
    ($($method:ident => $visit:ident: $ty:ty),*) => {$(
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
            visitor.$visit(self.parse::<$ty>(stringify!($ty))?)
        }
    )*};
}

impl<'de> de::Deserializer<'de> for &mut Reader<'_> {
    type Error = CustomIdError;

    fn deserialize_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value> {
        Err(CustomIdError::Invalid(
            "custom_id state needs a concrete type".into(),
        ))
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match &*self.next()? {
            "1" | "true" => visitor.visit_bool(true),
            "0" | "false" => visitor.visit_bool(false),
            other => Err(CustomIdError::Invalid(format!("`{other}` is not a bool"))),
        }
    }

    parse_number!(
        deserialize_i8 => visit_i8: i8,
        deserialize_i16 => visit_i16: i16,
        deserialize_i32 => visit_i32: i32,
        deserialize_i64 => visit_i64: i64,
        deserialize_u8 => visit_u8: u8,
        deserialize_u16 => visit_u16: u16,
        deserialize_u32 => visit_u32: u32,
        deserialize_u64 => visit_u64: u64,
        deserialize_f32 => visit_f32: f32,
        deserialize_f64 => visit_f64: f64,
        deserialize_char => visit_char: char
    );

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.next()? {
            Cow::Borrowed(field) => visitor.visit_str(field),
            Cow::Owned(field) => visitor.visit_string(field),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, _: V) -> Result<V::Value> {
        Writer::unsupported("bytes")
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, _: V) -> Result<V::Value> {
        Writer::unsupported("bytes")
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.next_is_empty() {
            self.next()?;
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.len()?;
        visitor.visit_seq(Items {
            reader: self,
            left: len,
        })
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(Items {
            reader: self,
            left: len,
        })
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_seq(Items {
            reader: self,
            left: len,
        })
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let len = self.len()?;
        visitor.visit_map(Items {
            reader: self,
            left: len,
        })
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_seq(Items {
            reader: self,
            left: fields.len(),
        })
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_enum(self)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_u32(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, _: V) -> Result<V::Value> {
        Err(CustomIdError::Invalid(
            "custom_id state cannot skip fields".into(),
        ))
    }
}

struct Items<'r, 'a> {
    reader: &'r mut Reader<'a>,
    left: usize,
}

impl<'de> de::SeqAccess<'de> for Items<'_, '_> {
    type Error = CustomIdError;

    fn next_element_seed<T: de::DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>> {
        if self.left == 0 {
            return Ok(None);
        }
        self.left -= 1;
        seed.deserialize(&mut *self.reader).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.left)
    }
}

impl<'de> de::MapAccess<'de> for Items<'_, '_> {
    type Error = CustomIdError;

    fn next_key_seed<K: de::DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        if self.left == 0 {
            return Ok(None);
        }
        self.left -= 1;
        seed.deserialize(&mut *self.reader).map(Some)
    }

    fn next_value_seed<V: de::DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value> {
        seed.deserialize(&mut *self.reader)
    }
}

impl<'de> de::EnumAccess<'de> for &mut Reader<'_> {
    type Error = CustomIdError;
    type Variant = Self;

    fn variant_seed<V: de::DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self)> {
        let index: u32 = self.parse("variant index")?;
        let value = seed.deserialize(index.into_deserializer())?;
        Ok((value, self))
    }
}

impl<'de> de::VariantAccess<'de> for &mut Reader<'_> {
    type Error = CustomIdError;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: de::DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        seed.deserialize(self)
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(Items {
            reader: self,
            left: len,
        })
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_seq(Items {
            reader: self,
            left: fields.len(),
        })
    }
}

/// `true` if `prefix` can start a `custom_id`: non-empty, at most 100 characters, without `:`
/// or `\`. Checked at compile time by the handler macros.
pub const fn valid_prefix(prefix: &str) -> bool {
    let bytes = prefix.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_LEN {
        return false;
    }
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':' || bytes[i] == b'\\' {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};

    use super::*;
    use crate::types::Snowflake;

    fn round_trip<T: Serialize + DeserializeOwned + PartialEq + fmt::Debug>(value: T) -> String {
        let id = encode("p", &value).unwrap();
        let (prefix, fields) = split(&id);
        assert_eq!(prefix, "p");
        assert_eq!(decode::<T>(fields).unwrap(), value, "{id}");
        id
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Page {
        user: Snowflake,
        page: u16,
        query: Option<String>,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Action {
        Up,
        Down,
        Set(i32),
        Move { x: i8, y: i8 },
    }

    #[test]
    fn encodes_compactly() {
        assert_eq!(round_trip(()), "p");
        assert_eq!(round_trip(5u32), "p:5");
        assert_eq!(round_trip((true, -3i64, 'x')), "p:1:-3:x");
        assert_eq!(round_trip("a:b\\c".to_string()), "p:a\\:b\\\\c");
        assert_eq!(
            round_trip(Page {
                user: Snowflake::from_u64(80351110224678912),
                page: 2,
                query: None
            }),
            "p:80351110224678912:2:"
        );
        assert_eq!(
            round_trip(Page {
                user: Snowflake::from_u64(1),
                page: 0,
                query: Some("hi there".into())
            }),
            "p:1:0:hi there"
        );
    }

    #[test]
    fn enums_sequences_and_maps() {
        assert_eq!(round_trip(Action::Down), "p:1");
        assert_eq!(round_trip(Action::Set(-7)), "p:2:-7");
        assert_eq!(round_trip(Action::Move { x: 1, y: -1 }), "p:3:1:-1");
        assert_eq!(round_trip(vec![1u8, 2, 3]), "p:3:1:2:3");
        assert_eq!(round_trip((Vec::<u8>::new(), 9u8)), "p:0:9");
        round_trip(BTreeMap::from([("a".to_string(), 1u8)]));
        round_trip(Some(Some(4u8)));
        round_trip(1.5f64);
    }

    #[test]
    fn reads_escaped_fields() {
        assert_eq!(round_trip(("a:b".to_string(), 1u8)), r"p:a\:b:1");
        assert_eq!(
            round_trip((Some(r"\".to_string()), None::<u8>, 2u8)),
            r"p:\\::2"
        );
        assert_eq!(decode::<String>(Some("")).unwrap(), "");
        assert_eq!(decode::<Option<String>>(Some("")).unwrap(), None);
    }

    #[test]
    fn rejects_mismatches() {
        assert!(decode::<u8>(Some("300")).is_err());
        assert!(decode::<u8>(None).is_err());
        assert_eq!(
            decode::<u8>(Some("1:2:3")).unwrap_err().to_string(),
            "custom_id: 2 unexpected trailing fields"
        );
        assert!(decode::<bool>(Some("maybe")).is_err());
        assert!(decode::<Action>(Some("9")).is_err());
    }

    #[test]
    fn enforces_the_length_limit() {
        assert_eq!(
            encode("p", &"x".repeat(99)).unwrap_err(),
            CustomIdError::TooLong(101)
        );
        assert_eq!(encode("p", &"x".repeat(98)).unwrap().len(), 100);
    }

    #[test]
    fn splits_on_the_first_unescaped_separator() {
        assert_eq!(split("abc"), ("abc", None));
        assert_eq!(split("abc:"), ("abc", Some("")));
        assert_eq!(split("a\\:b:c"), ("a\\:b", Some("c")));
    }

    #[test]
    fn validates_prefixes() {
        assert!(valid_prefix("counter"));
        assert!(!valid_prefix(""));
        assert!(!valid_prefix("a:b"));
        assert!(!valid_prefix(&"x".repeat(101)));
    }
}
