use serde::{de::Visitor, Deserialize};

use crate::{
    models::{Permissions, Snowflake},
    Mentionable,
};

/// [Role Object](https://discord.com/developers/docs/topics/permissions#role-object)
#[derive(Debug, Deserialize)]
pub struct Role {
    /// role id
    pub id: Snowflake,

    /// role name
    pub name: String,

    /// integer representation of hexadecimal color code
    pub color: i32,

    /// if this role is pinned in the user listing
    pub hoist: bool,

    /// role [icon hash](https://discord.com/developers/docs/reference#image-formatting)
    pub icon: Option<String>,

    /// role unicode emoji
    pub unicode_emoji: Option<String>,

    /// position of this role
    pub position: i32,

    /// permission bit set
    pub permissions: Permissions,

    /// whether this role is managed by an integration
    pub managed: bool,

    /// whether this role is mentionable
    pub mentionable: bool,

    /// the tags this role has
    pub tags: Option<RoleTags>,
}

impl Mentionable for Role {
    fn to_mention(&self) -> String {
        format!("<@&{}>", self.id)
    }
}

/// [Role Subscription Data Object](https://discord.com/developers/docs/resources/channel#role-subscription-data-object)
#[derive(Debug, Deserialize)]
pub struct RoleSubscriptionData {
    /// the id of the sku and listing that the user is subscribed to
    pub role_subscription_listing_id: Snowflake,

    /// the name of the tier that the user is subscribed to
    pub tier_name: String,

    /// the cumulative number of months that the user has been subscribed for
    pub total_months_subscribed: i32,

    /// whether this notification is for a renewal rather than a new purchase
    pub is_renewal: bool,
}

/// [Role Tags Structure](https://discord.com/developers/docs/topics/permissions#role-object-role-tags-structure)
#[derive(Debug)]
pub struct RoleTags {
    /// the id of the bot this role belongs to
    pub bot_id: Option<Snowflake>,

    /// the id of the integration this role belongs to
    pub integration_id: Option<Snowflake>,

    /// whether this is the guild's Booster role
    pub premium_subscriber: bool,

    /// the id of this role's subscription sku and listing
    pub subscription_listing_id: Option<Snowflake>,

    /// whether this role is available for purchase
    pub available_for_purchase: bool,

    /// whether this role is a guild's linked role
    pub guild_connections: bool,
}

impl<'de> Deserialize<'de> for RoleTags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &'static [&'static str] = &[
            "bot_id",
            "integration_id",
            "premium_subscriber",
            "subscription_listing_id",
            "available_for_purchase",
            "guild_connections",
        ];

        enum Field {
            BotId,
            IntegrationId,
            PremiumSubscriber,
            SubscriptionListingId,
            AvailableForPurchase,
            GuildConnections,
        }

        // This part could also be generated independently by:
        //
        //    #[derive(Deserialize)]
        //    #[serde(field_identifier, rename_all = "lowercase")]
        //    enum Field { Secs, Nanos }
        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("a field from RoleTags")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "bot_id" => Ok(Field::BotId),
                            "integration_id" => Ok(Field::IntegrationId),
                            "premium_subscriber" => Ok(Field::PremiumSubscriber),
                            "subscription_listing_id" => Ok(Field::SubscriptionListingId),
                            "available_for_purchase" => Ok(Field::AvailableForPurchase),
                            "guild_connections" => Ok(Field::GuildConnections),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct RoleTagsVisitor;

        impl<'de> Visitor<'de> for RoleTagsVisitor {
            type Value = RoleTags;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("struct RoleTags")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut bot_id: Option<Snowflake> = None;
                let mut integration_id: Option<Snowflake> = None;
                let mut premium_subscriber: bool = false;
                let mut subscription_listing_id: Option<Snowflake> = None;
                let mut available_for_purchase: bool = false;
                let mut guild_connections: bool = false;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::AvailableForPurchase => {
                            _ = map.next_value()?;
                            available_for_purchase = true;
                        }
                        Field::BotId => {
                            bot_id = map.next_value()?;
                        }
                        Field::IntegrationId => {
                            integration_id = map.next_value()?;
                        }
                        Field::PremiumSubscriber => {
                            _ = map.next_value()?;
                            premium_subscriber = true;
                        }
                        Field::SubscriptionListingId => {
                            subscription_listing_id = map.next_value()?;
                        }
                        Field::GuildConnections => {
                            _ = map.next_value()?;
                            guild_connections = true;
                        }
                    }
                }

                Ok(RoleTags {
                    bot_id,
                    integration_id,
                    premium_subscriber,
                    subscription_listing_id,
                    available_for_purchase,
                    guild_connections,
                })
            }
        }

        deserializer.deserialize_struct("RoleTags", FIELDS, RoleTagsVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn role_tags_deserializes_all_true() {
        let json = r#"{
			"bot_id": "282265607313817601",
            "integration_id": "282265607313817601",
            "subscription_listing_id": "282265607313817601",
            "premium_subscriber": null,
            "available_for_purchase": null,
            "guild_connections": null
		}"#;

        let result = serde_json::from_str::<RoleTags>(json);

        assert!(result.is_ok());

        let role_tags = result.unwrap();

        assert_eq!(role_tags.premium_subscriber, true);
        assert_eq!(role_tags.available_for_purchase, true);
        assert_eq!(role_tags.guild_connections, true);
    }

    #[test]
    pub fn role_tags_deserializes_all_false() {
        let json = r#"{
			"bot_id": "282265607313817601",
            "integration_id": "282265607313817601",
            "subscription_listing_id": "282265607313817601"
		}"#;

        let result = serde_json::from_str::<RoleTags>(json);

        assert!(result.is_ok());

        let role_tags = result.unwrap();

        assert_eq!(role_tags.premium_subscriber, false);
        assert_eq!(role_tags.available_for_purchase, false);
        assert_eq!(role_tags.guild_connections, false);
    }
}
