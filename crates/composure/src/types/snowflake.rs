use std::{fmt::Debug, str::FromStr};

use serde::{Deserialize, Serialize, de::Visitor};

const DISCORD_EPOCH: u64 = 1420070400000;

const WORKER_BITS: u64 = 0x3E0000;
const PROCESS_ID_BITS: u64 = 0x1F000;
const INCREMENT_BITS: u64 = 0xFFF;

const TIMESTAMP_SHIFT: u8 = 22;
const WORKER_SHIFT: u8 = 17;
const PROCESS_ID_SHIFT: u8 = 12;

/// A Discord id: a 64-bit integer carrying its creation time, serialized as a string
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Snowflake(u64);

impl Snowflake {
    /// The id with this numeric value
    pub const fn from_u64(snowflake: u64) -> Self {
        Snowflake(snowflake)
    }

    /// The numeric value
    pub const fn to_u64(&self) -> u64 {
        self.0
    }

    /// Creation time in milliseconds since the Unix epoch
    pub const fn timestamp(&self) -> u64 {
        (self.0 >> TIMESTAMP_SHIFT) + DISCORD_EPOCH
    }

    /// The internal worker that generated the id
    pub const fn worker_id(&self) -> u8 {
        ((self.0 & WORKER_BITS) >> WORKER_SHIFT) as u8
    }

    /// The internal process that generated the id
    pub const fn internal_process_id(&self) -> u8 {
        ((self.0 & PROCESS_ID_BITS) >> PROCESS_ID_SHIFT) as u8
    }

    /// Incremented for every id generated on that process
    pub const fn increment(&self) -> u16 {
        (self.0 & INCREMENT_BITS) as u16
    }
}

impl From<u64> for Snowflake {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl FromStr for Snowflake {
    type Err = <u64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl From<Snowflake> for u64 {
    fn from(val: Snowflake) -> Self {
        val.0
    }
}

impl Debug for Snowflake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Snowflake").field(&self.0).finish()
    }
}

impl std::fmt::Display for Snowflake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl<'de> Deserialize<'de> for Snowflake {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SnowflakeVisitor;

        impl<'de> Visitor<'de> for SnowflakeVisitor {
            type Value = Snowflake;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an id as a string of digits")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Snowflake::from_str(v).map_err(|_| {
                    serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self)
                })
            }
        }

        deserializer.deserialize_str(SnowflakeVisitor)
    }
}

impl Serialize for Snowflake {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(&self.0)
    }
}

/// Objects keyed by id, as Discord sends resolved data. These hold a handful of entries, so a
/// list is faster to build and search than a hash map, and smaller in the bundle.
#[derive(Debug, Clone)]
pub struct SnowflakeMap<T>(Vec<(Snowflake, T)>);

impl<T> SnowflakeMap<T> {
    /// The object with id `id`
    pub fn get(&self, id: &Snowflake) -> Option<&T> {
        self.0
            .iter()
            .find(|(key, _)| key == id)
            .map(|(_, value)| value)
    }

    /// Whether an object has id `id`
    pub fn contains_key(&self, id: &Snowflake) -> bool {
        self.get(id).is_some()
    }

    /// How many objects there are
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether there are none
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Every id and object, in the order received
    pub fn iter(&self) -> impl Iterator<Item = (&Snowflake, &T)> {
        self.0.iter().map(|(key, value)| (key, value))
    }

    /// Every id
    pub fn keys(&self) -> impl Iterator<Item = &Snowflake> {
        self.0.iter().map(|(key, _)| key)
    }

    /// Every object
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.0.iter().map(|(_, value)| value)
    }
}

impl<T> Default for SnowflakeMap<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T> FromIterator<(Snowflake, T)> for SnowflakeMap<T> {
    fn from_iter<I: IntoIterator<Item = (Snowflake, T)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<T> std::ops::Index<&Snowflake> for SnowflakeMap<T> {
    type Output = T;

    fn index(&self, id: &Snowflake) -> &T {
        self.get(id).expect("no entry for id")
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for SnowflakeMap<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MapVisitor<T>(std::marker::PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for MapVisitor<T> {
            type Value = SnowflakeMap<T>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a map keyed by id")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(1).min(25));
                while let Some(entry) = map.next_entry()? {
                    entries.push(entry);
                }
                Ok(SnowflakeMap(entries))
            }
        }

        deserializer.deserialize_map(MapVisitor(std::marker::PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn snowflake_is_one_word() {
        assert_eq!(std::mem::size_of::<Snowflake>(), 8);
        let snowflake = Snowflake::from(282265607313817601);
        assert_eq!(
            serde_json::to_string(&snowflake).unwrap(),
            r#""282265607313817601""#
        );
        assert_eq!(
            (
                snowflake.worker_id(),
                snowflake.internal_process_id(),
                snowflake.increment()
            ),
            (0, 0, 1)
        );
    }

    #[test]
    pub fn snowflake_map_reads_an_object_keyed_by_id() {
        let map: SnowflakeMap<String> =
            serde_json::from_str(r#"{ "1": "one", "22": "twenty-two" }"#).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(
            map.get(&Snowflake::from(22)).map(String::as_str),
            Some("twenty-two")
        );
        assert_eq!(map[&Snowflake::from(1)], "one");
        assert!(!map.contains_key(&Snowflake::from(3)));
        assert!(serde_json::from_str::<SnowflakeMap<String>>(r#"{ "x": "bad id" }"#).is_err());
    }

    #[test]
    pub fn timestamp_correct() {
        let snowflake: u64 = 282265607313817601;
        let snowflake = Snowflake::from(snowflake);

        assert_eq!(snowflake.timestamp(), 1487367765025);
    }

    #[test]
    pub fn timestamp_correct_from_str() {
        let snowflake = "282265607313817601";
        let snowflake = Snowflake::from_str(snowflake);

        assert!(snowflake.is_ok());

        let snowflake = snowflake.unwrap();

        assert_eq!(snowflake.timestamp(), 1487367765025);
    }

    #[test]
    pub fn to_u64_works() {
        let snowflake_id: u64 = 282265607313817601;
        let snowflake = Snowflake::from(snowflake_id);

        let back_to_u64 = snowflake.to_u64();

        assert_eq!(snowflake_id, back_to_u64);
    }

    #[test]
    pub fn deserialize_works() {
        let snowflake_id = r#""282265607313817601""#;

        let snowflake = serde_json::from_str::<Snowflake>(snowflake_id);

        assert!(snowflake.is_ok());

        let snowflake = snowflake.unwrap();

        assert_eq!(snowflake.timestamp(), 1487367765025);
        let trimmed = &snowflake_id[1..snowflake_id.len() - 1];
        assert_eq!(trimmed, snowflake.to_string().as_str());
    }
}
