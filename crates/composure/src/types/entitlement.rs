use serde::Deserialize;

use crate::types::Snowflake;

/// [Entitlement](https://discord.com/developers/docs/resources/entitlement#entitlement-object):
/// a user's or guild's access to a premium SKU
#[derive(Debug, Clone, Deserialize)]
pub struct Entitlement {
    /// The entitlement's id
    pub id: Snowflake,

    /// The SKU it grants
    pub sku_id: Snowflake,

    /// The application the SKU belongs to
    pub application_id: Snowflake,

    /// The user granted access, for user subscriptions and purchases
    pub user_id: Option<Snowflake>,

    /// The guild granted access, for guild subscriptions
    pub guild_id: Option<Snowflake>,

    /// [Entitlement type](https://discord.com/developers/docs/resources/entitlement#entitlement-object-entitlement-types):
    /// 1 purchase, 2 premium subscription, 3 developer gift, 4 test mode purchase, 5 free
    /// purchase, 6 user gift, 7 premium purchase, 8 application subscription
    #[serde(rename = "type")]
    pub kind: u8,

    /// Deleted (refunded, revoked)
    pub deleted: bool,

    /// Start of validity (ISO 8601); absent for test entitlements
    pub starts_at: Option<String>,

    /// End of validity (ISO 8601); absent for test entitlements and permanent ones
    pub ends_at: Option<String>,

    /// For consumable SKUs, whether it was consumed
    pub consumed: Option<bool>,
}
