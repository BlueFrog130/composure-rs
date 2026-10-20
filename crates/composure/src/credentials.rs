//! What identifies and authorizes the Discord application (ADR 0014). Composure never loads
//! configuration itself: an Adapter (or the bot) builds [`Credentials`] from wherever it keeps
//! them, by the conventional names with [`Credentials::from_lookup`], or through serde with any
//! loader (`config`, `figment`, `envy`, ...).

use std::fmt;

use serde::{Deserialize, Deserializer};

use crate::types::Snowflake;

/// Secret / environment variable holding the application's hex public key
pub const PUBLIC_KEY_VAR: &str = "DISCORD_PUBLIC_KEY";
/// Secret / environment variable holding the bot token, for the Bot Client
pub const BOT_TOKEN_VAR: &str = "DISCORD_TOKEN";
/// Variable holding the application id, for a Bot Client outside Interactions
pub const APPLICATION_ID_VAR: &str = "DISCORD_APPLICATION_ID";

/// The application's public key (required, verifies Interactions), and optionally the bot token
/// (for [`Ctx::bot`](crate::command::Ctx::bot)) and the application id (for a Bot Client outside
/// Interactions).
///
/// Build it in code, from the conventional variables with [`from_lookup`](Self::from_lookup), or
/// deserialize it: fields `public_key`, `bot_token` (or `token`), `application_id`.
///
/// ```
/// # fn main() -> Result<(), composure::CredentialsError> {
/// use composure::Credentials;
///
/// let key = "a".repeat(64);
/// let credentials = Credentials::new(&key)?.with_bot_token("token");
/// assert_eq!(credentials.bot_token(), Some("token"));
///
/// // `config`: `settings.get::<Credentials>("discord")`, `envy`:
/// // `envy::prefixed("DISCORD_").from_env::<Credentials>()`, or any serde format
/// let credentials: Credentials = serde_json::from_str(&format!(
///     r#"{{ "public_key": "{key}", "token": "token", "application_id": 123 }}"#
/// )).unwrap();
/// assert_eq!(credentials.application_id(), Some(123u64.into()));
/// # Ok(())
/// # }
/// ```
///
/// `Debug` never shows the bot token.
#[derive(Clone, Deserialize)]
pub struct Credentials {
    #[serde(deserialize_with = "public_key")]
    public_key: String,
    #[serde(default, alias = "token", deserialize_with = "non_empty")]
    bot_token: Option<String>,
    #[serde(default, deserialize_with = "application_id")]
    application_id: Option<Snowflake>,
}

impl Credentials {
    /// With the application's public key (64 hex characters), no bot token or application id
    pub fn new(public_key: &str) -> Result<Self, CredentialsError> {
        check_public_key(public_key).map_err(|reason| CredentialsError::Invalid {
            name: "public key",
            reason: reason.into(),
        })?;
        Ok(Self {
            public_key: public_key.to_owned(),
            bot_token: None,
            application_id: None,
        })
    }

    /// From the conventional variables, `lookup` reading one by name: `DISCORD_PUBLIC_KEY`
    /// (required), `DISCORD_TOKEN` and `DISCORD_APPLICATION_ID` (optional). An empty value is
    /// unset.
    ///
    /// ```
    /// # use composure::Credentials;
    /// let credentials = Credentials::from_lookup(|name| std::env::var(name).ok());
    /// ```
    pub fn from_lookup(
        mut lookup: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, CredentialsError> {
        let mut read = |name| lookup(name).filter(|value| !value.is_empty());
        let public_key = read(PUBLIC_KEY_VAR).ok_or(CredentialsError::Missing(PUBLIC_KEY_VAR))?;
        check_public_key(&public_key).map_err(|reason| CredentialsError::Invalid {
            name: PUBLIC_KEY_VAR,
            reason: reason.into(),
        })?;
        let application_id = match read(APPLICATION_ID_VAR) {
            Some(id) => Some(id.parse().map_err(|_| CredentialsError::Invalid {
                name: APPLICATION_ID_VAR,
                reason: "not an id".into(),
            })?),
            None => None,
        };
        Ok(Self {
            public_key,
            bot_token: read(BOT_TOKEN_VAR),
            application_id,
        })
    }

    /// With the bot token, without the `Bot ` prefix
    pub fn with_bot_token(mut self, token: impl Into<String>) -> Self {
        self.bot_token = Some(token.into());
        self
    }

    /// With the application id
    pub fn with_application_id(mut self, id: Snowflake) -> Self {
        self.application_id = Some(id);
        self
    }

    /// The application's public key, as hex
    pub fn public_key(&self) -> &str {
        &self.public_key
    }

    /// The bot token, without the `Bot ` prefix
    pub fn bot_token(&self) -> Option<&str> {
        self.bot_token.as_deref()
    }

    /// The application id
    pub fn application_id(&self) -> Option<Snowflake> {
        self.application_id
    }

    /// A Bot Client over `transport`, for REST outside Interactions; `None` without both a bot
    /// token and an application id
    #[cfg(feature = "rest")]
    pub fn bot_client<T: crate::rest::Transport>(
        &self,
        transport: T,
    ) -> Option<crate::rest::BotClient<T>> {
        Some(crate::rest::BotClient::new(
            transport,
            self.bot_token.as_deref()?,
            self.application_id?,
        ))
    }
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("public_key", &self.public_key)
            .field("bot_token", &self.bot_token.as_ref().map(|_| "<redacted>"))
            .field("application_id", &self.application_id)
            .finish()
    }
}

/// 64 hex characters; whether they are a valid key is up to the Adapter, which imports it
fn check_public_key(key: &str) -> Result<(), &'static str> {
    if key.len() == 64 && key.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err("not 64 hex characters")
    }
}

fn public_key<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let key = String::deserialize(deserializer)?;
    check_public_key(&key).map_err(serde::de::Error::custom)?;
    Ok(key)
}

/// A string (as Discord and environment variables have it) or an integer (TOML, YAML); empty is
/// unset
fn application_id<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Snowflake>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Id {
        Number(u64),
        Text(String),
    }
    match Option::<Id>::deserialize(deserializer)? {
        None => Ok(None),
        Some(Id::Number(id)) => Ok(Some(id.into())),
        Some(Id::Text(id)) if id.is_empty() => Ok(None),
        Some(Id::Text(id)) => id
            .parse()
            .map(Some)
            .map_err(|_| serde::de::Error::custom("application_id is not an id")),
    }
}

fn non_empty<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.filter(|value| !value.is_empty()))
}

/// Why [`Credentials`] could not be built
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialsError {
    /// A required value is not set
    Missing(&'static str),
    /// A value is set but unusable
    Invalid {
        /// The variable (or value) at fault
        name: &'static str,
        /// What is wrong with it
        reason: String,
    },
}

impl fmt::Display for CredentialsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing(name) => write!(f, "{name} is not set"),
            Self::Invalid { name, reason } => write!(f, "{name} is invalid: {reason}"),
        }
    }
}

impl std::error::Error for CredentialsError {}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789ABCDEF";

    fn vars<'a>(vars: &'a [(&'a str, &'a str)]) -> impl FnMut(&str) -> Option<String> + 'a {
        |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    #[test]
    fn from_lookup_reads_conventional_names() {
        let credentials = Credentials::from_lookup(vars(&[
            (PUBLIC_KEY_VAR, KEY),
            (BOT_TOKEN_VAR, "t"),
            (APPLICATION_ID_VAR, "42"),
        ]))
        .unwrap();
        assert_eq!(credentials.public_key(), KEY);
        assert_eq!(credentials.bot_token(), Some("t"));
        assert_eq!(credentials.application_id(), Some(Snowflake::from(42u64)));
    }

    #[test]
    fn from_lookup_treats_empty_as_unset() {
        let credentials =
            Credentials::from_lookup(vars(&[(PUBLIC_KEY_VAR, KEY), (BOT_TOKEN_VAR, "")])).unwrap();
        assert_eq!(credentials.bot_token(), None);
        assert_eq!(
            Credentials::from_lookup(vars(&[(PUBLIC_KEY_VAR, "")])).unwrap_err(),
            CredentialsError::Missing(PUBLIC_KEY_VAR)
        );
    }

    #[test]
    fn from_lookup_rejects_bad_values() {
        assert!(matches!(
            Credentials::from_lookup(vars(&[(PUBLIC_KEY_VAR, "abc")])),
            Err(CredentialsError::Invalid {
                name: PUBLIC_KEY_VAR,
                ..
            })
        ));
        assert!(matches!(
            Credentials::from_lookup(vars(&[(PUBLIC_KEY_VAR, KEY), (APPLICATION_ID_VAR, "x")])),
            Err(CredentialsError::Invalid {
                name: APPLICATION_ID_VAR,
                ..
            })
        ));
    }

    #[test]
    fn deserializes_with_token_alias_and_numeric_id() {
        let json = format!(r#"{{"public_key":"{KEY}","token":"t","application_id":42,"other":1}}"#);
        let credentials: Credentials = serde_json::from_slice(json.as_bytes()).unwrap();
        assert_eq!(credentials.bot_token(), Some("t"));
        assert_eq!(credentials.application_id(), Some(Snowflake::from(42u64)));

        let json = format!(r#"{{"public_key":"{KEY}","bot_token":"","application_id":"42"}}"#);
        let credentials: Credentials = serde_json::from_slice(json.as_bytes()).unwrap();
        assert_eq!(credentials.bot_token(), None);
        assert_eq!(credentials.application_id(), Some(Snowflake::from(42u64)));
    }

    #[test]
    fn deserialize_rejects_bad_public_key() {
        assert!(serde_json::from_slice::<Credentials>(br#"{"public_key":"nope"}"#).is_err());
        assert!(serde_json::from_slice::<Credentials>(br#"{}"#).is_err());
    }

    #[test]
    fn debug_redacts_token() {
        let credentials = Credentials::new(KEY)
            .unwrap()
            .with_bot_token("secret-token");
        let debug = format!("{credentials:?}");
        assert!(!debug.contains("secret-token"));
        assert!(debug.contains("<redacted>"));
    }
}
