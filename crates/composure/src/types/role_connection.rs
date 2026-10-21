//! [Linked Roles](https://discord.com/developers/docs/resources/application-role-connection-metadata):
//! the metadata an app declares, which servers turn into role requirements

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// [Application Role Connection Metadata](https://discord.com/developers/docs/resources/application-role-connection-metadata#application-role-connection-metadata-object):
/// one value an app reports per user, and how servers compare it
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ApplicationRoleConnectionMetadata {
    /// How the user's value is compared with the server's
    #[serde(rename = "type")]
    pub t: RoleConnectionMetadataType,

    /// The key of the user's value, `a-z`, `0-9` or `_`, 1-50 characters
    pub key: String,

    /// Shown to server admins, 1-100 characters
    pub name: String,

    /// `name` per locale
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,

    /// Shown to server admins, 1-200 characters
    pub description: String,

    /// `description` per locale
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,
}

/// [Metadata Type](https://discord.com/developers/docs/resources/application-role-connection-metadata#application-role-connection-metadata-object-application-role-connection-metadata-type):
/// how a user's value is compared with the one a server sets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum RoleConnectionMetadataType {
    /// The user's integer is at most the server's
    IntegerLessThanOrEqual = 1,
    /// The user's integer is at least the server's
    IntegerGreaterThanOrEqual = 2,
    /// The user's integer equals the server's
    IntegerEqual = 3,
    /// The user's integer differs from the server's
    IntegerNotEqual = 4,
    /// The user's date (ISO 8601) is at least the server's number of days ago
    DatetimeLessThanOrEqual = 5,
    /// The user's date (ISO 8601) is at most the server's number of days ago
    DatetimeGreaterThanOrEqual = 6,
    /// The user's boolean (`1`/`0`) equals the server's
    BooleanEqual = 7,
    /// The user's boolean (`1`/`0`) differs from the server's
    BooleanNotEqual = 8,
}
