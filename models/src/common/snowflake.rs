use std::{hash::Hash, str::FromStr};

use serde::{de::Visitor, Deserialize, Serialize};

const DISCORD_EPOCH: u64 = 1420070400000;

const WORKER_BITS: u64 = 0x3E0000;
const PROCESS_ID_BITS: u64 = 0x1F000;
const INCREMENT_BITS: u64 = 0xFFF;

const TIMESTAMP_SHIFT: u8 = 22;
const WORKER_SHIFT: u8 = 17;
const PROCESS_ID_SHIFT: u8 = 12;

#[derive(Debug, Eq)]
pub struct Snowflake {
    pub timestamp: u64,
    worker_id: u8,
    internal_process_id: u8,
    increment: u16,
}

impl Snowflake {
    pub fn from_u64(snowflake: u64) -> Self {
        Snowflake {
            timestamp: (snowflake >> TIMESTAMP_SHIFT) + DISCORD_EPOCH,
            worker_id: ((snowflake & WORKER_BITS) >> WORKER_SHIFT) as u8,
            internal_process_id: ((snowflake & PROCESS_ID_BITS) >> PROCESS_ID_SHIFT) as u8,
            increment: (snowflake & INCREMENT_BITS) as u16,
        }
    }

    pub fn to_u64(&self) -> u64 {
        let mut snowflake: u64 = 0;

        snowflake |= self.timestamp - DISCORD_EPOCH << TIMESTAMP_SHIFT;
        snowflake |= (self.worker_id as u64) << WORKER_SHIFT;
        snowflake |= (self.internal_process_id as u64) << PROCESS_ID_SHIFT;
        snowflake |= self.increment as u64;

        snowflake
    }
}

impl Default for Snowflake {
    fn default() -> Self {
        Self {
            timestamp: DISCORD_EPOCH,
            worker_id: Default::default(),
            internal_process_id: Default::default(),
            increment: Default::default(),
        }
    }
}

impl PartialEq for Snowflake {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
            && self.worker_id == other.worker_id
            && self.internal_process_id == other.internal_process_id
            && self.increment == other.increment
    }
}

impl Hash for Snowflake {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_u64().hash(state);
    }
}

impl From<u64> for Snowflake {
    fn from(value: u64) -> Self {
        Self::from_u64(value)
    }
}

impl FromStr for Snowflake {
    type Err = <u64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_u64(s.parse()?))
    }
}

impl Into<u64> for Snowflake {
    fn into(self) -> u64 {
        self.to_u64()
    }
}

impl ToString for Snowflake {
    fn to_string(&self) -> String {
        self.to_u64().to_string()
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
                formatter.write_str("SnowflakeVisitor")
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
        serializer.serialize_str(self.to_string().as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn timestamp_correct() {
        let snowflake: u64 = 282265607313817601;
        let snowflake = Snowflake::from(snowflake);

        assert_eq!(snowflake.timestamp, 1487367765025);
    }

    #[test]
    pub fn timestamp_correct_from_str() {
        let snowflake = "282265607313817601";
        let snowflake = Snowflake::from_str(snowflake);

        assert!(snowflake.is_ok());

        let snowflake = snowflake.unwrap();

        assert_eq!(snowflake.timestamp, 1487367765025);
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

        assert_eq!(snowflake.timestamp, 1487367765025);
        let trimmed = &snowflake_id[1..snowflake_id.len() - 1];
        assert_eq!(trimmed, snowflake.to_string().as_str());
    }
}
