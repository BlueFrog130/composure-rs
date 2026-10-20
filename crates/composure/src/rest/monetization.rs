//! [Monetization](https://discord.com/developers/docs/resources/entitlement): entitlements and SKU
//! subscriptions

use serde::Serialize;

use crate::types::{Entitlement, Snowflake, Subscription};

use super::{
    Body, BotClient, Response, Result, Transport,
    route::{Query, routes},
};

routes! {
    LIST_ENTITLEMENTS = Get "/applications/{application_id}/entitlements";
    GET_ENTITLEMENT = Get "/applications/{application_id}/entitlements/{entitlement_id}";
    CONSUME_ENTITLEMENT = Post "/applications/{application_id}/entitlements/{entitlement_id}/consume";
    CREATE_TEST_ENTITLEMENT = Post "/applications/{application_id}/entitlements";
    DELETE_TEST_ENTITLEMENT = Delete "/applications/{application_id}/entitlements/{entitlement_id}";
    LIST_SKU_SUBSCRIPTIONS = Get "/skus/{sku_id}/subscriptions";
    GET_SKU_SUBSCRIPTION = Get "/skus/{sku_id}/subscriptions/{subscription_id}";
}

/// Filters and paging of [`BotClient::list_entitlements`], by entitlement id
#[derive(Debug, Clone, Copy, Default)]
pub struct ListEntitlements<'a> {
    /// Only this user's
    pub user_id: Option<Snowflake>,
    /// Only for these SKUs
    pub sku_ids: &'a [Snowflake],
    /// Only this guild's
    pub guild_id: Option<Snowflake>,
    /// Entitlements before this id
    pub before: Option<Snowflake>,
    /// Entitlements after this id
    pub after: Option<Snowflake>,
    /// 1-100, default 100
    pub limit: Option<u8>,
    /// Leave out entitlements that ended
    pub exclude_ended: bool,
    /// Leave out deleted entitlements
    pub exclude_deleted: bool,
}

/// Owner of a test entitlement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitlementOwner {
    /// A guild
    Guild(Snowflake),
    /// A user
    User(Snowflake),
}

#[derive(Serialize)]
struct CreateTestEntitlement {
    sku_id: Snowflake,
    owner_id: Snowflake,
    owner_type: u8,
}

impl<T: Transport> BotClient<T> {
    /// [List Entitlements](https://discord.com/developers/docs/resources/entitlement#list-entitlements)
    pub async fn list_entitlements(
        &self,
        query: &ListEntitlements<'_>,
    ) -> Result<Response<Vec<Entitlement>>> {
        let mut url = self.rest.url(LIST_ENTITLEMENTS, &[&self.application_id]);
        let sku_ids = (!query.sku_ids.is_empty()).then(|| {
            query
                .sku_ids
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        });
        Query::new(&mut url)
            .push_opt("user_id", query.user_id)
            .push_opt("sku_ids", sku_ids)
            .push_opt("guild_id", query.guild_id)
            .push_opt("before", query.before)
            .push_opt("after", query.after)
            .push_opt("limit", query.limit)
            .push_opt("exclude_ended", query.exclude_ended.then_some(true))
            .push_opt("exclude_deleted", query.exclude_deleted.then_some(true));
        self.rest
            .call(LIST_ENTITLEMENTS, url, Body::NONE, None)
            .await
    }

    /// [Get Entitlement](https://discord.com/developers/docs/resources/entitlement#get-entitlement)
    pub async fn get_entitlement(
        &self,
        entitlement_id: Snowflake,
    ) -> Result<Response<Entitlement>> {
        let url = self
            .rest
            .url(GET_ENTITLEMENT, &[&self.application_id, &entitlement_id]);
        self.rest.call(GET_ENTITLEMENT, url, Body::NONE, None).await
    }

    /// [Consume an Entitlement](https://discord.com/developers/docs/resources/entitlement#consume-an-entitlement)
    /// for a one-time purchase consumable
    pub async fn consume_entitlement(&self, entitlement_id: Snowflake) -> Result<()> {
        let url = self.rest.url(
            CONSUME_ENTITLEMENT,
            &[&self.application_id, &entitlement_id],
        );
        self.rest
            .call_empty(CONSUME_ENTITLEMENT, url, Body::NONE, None)
            .await
    }

    /// [Create Test Entitlement](https://discord.com/developers/docs/resources/entitlement#create-test-entitlement):
    /// grant `sku_id` without payment, for testing
    pub async fn create_test_entitlement(
        &self,
        sku_id: Snowflake,
        owner: EntitlementOwner,
    ) -> Result<Response<Entitlement>> {
        let url = self
            .rest
            .url(CREATE_TEST_ENTITLEMENT, &[&self.application_id]);
        let (owner_id, owner_type) = match owner {
            EntitlementOwner::Guild(id) => (id, 1),
            EntitlementOwner::User(id) => (id, 2),
        };
        let body = CreateTestEntitlement {
            sku_id,
            owner_id,
            owner_type,
        };
        self.rest
            .call(CREATE_TEST_ENTITLEMENT, url, Body::Json(&body), None)
            .await
    }

    /// [Delete Test Entitlement](https://discord.com/developers/docs/resources/entitlement#delete-test-entitlement)
    pub async fn delete_test_entitlement(&self, entitlement_id: Snowflake) -> Result<()> {
        let url = self.rest.url(
            DELETE_TEST_ENTITLEMENT,
            &[&self.application_id, &entitlement_id],
        );
        self.rest
            .call_empty(DELETE_TEST_ENTITLEMENT, url, Body::NONE, None)
            .await
    }

    /// [List SKU Subscriptions](https://discord.com/developers/docs/resources/subscription#list-sku-subscriptions)
    /// of one user (`user_id` is required for bots), paged by subscription id
    pub async fn list_sku_subscriptions(
        &self,
        sku_id: Snowflake,
        user_id: Snowflake,
        before: Option<Snowflake>,
        after: Option<Snowflake>,
        limit: Option<u8>,
    ) -> Result<Response<Vec<Subscription>>> {
        let mut url = self.rest.url(LIST_SKU_SUBSCRIPTIONS, &[&sku_id]);
        Query::new(&mut url)
            .push_opt("before", before)
            .push_opt("after", after)
            .push_opt("limit", limit)
            .push("user_id", user_id);
        self.rest
            .call(LIST_SKU_SUBSCRIPTIONS, url, Body::NONE, None)
            .await
    }

    /// [Get SKU Subscription](https://discord.com/developers/docs/resources/subscription#get-sku-subscription)
    pub async fn get_sku_subscription(
        &self,
        sku_id: Snowflake,
        subscription_id: Snowflake,
    ) -> Result<Response<Subscription>> {
        let url = self
            .rest
            .url(GET_SKU_SUBSCRIPTION, &[&sku_id, &subscription_id]);
        self.rest
            .call(GET_SKU_SUBSCRIPTION, url, Body::NONE, None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::json;

    use super::super::{Method::*, testing::MockTransport};
    use super::*;

    #[test]
    fn routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        let (e, sku, u) = (
            Snowflake::from_u64(5),
            Snowflake::from_u64(6),
            Snowflake::from_u64(7),
        );
        block_on(c.list_entitlements(&ListEntitlements {
            user_id: Some(u),
            sku_ids: &[sku, e],
            exclude_ended: true,
            ..Default::default()
        }))
        .unwrap();
        block_on(c.get_entitlement(e)).unwrap();
        block_on(c.consume_entitlement(e)).unwrap();
        block_on(c.create_test_entitlement(sku, EntitlementOwner::User(u))).unwrap();
        block_on(c.delete_test_entitlement(e)).unwrap();
        block_on(c.list_sku_subscriptions(sku, u, None, None, Some(10))).unwrap();
        block_on(c.get_sku_subscription(sku, e)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (
                    Get,
                    "/applications/1/entitlements?user_id=7&sku_ids=6%2C5&exclude_ended=true"
                        .into()
                ),
                (Get, "/applications/1/entitlements/5".into()),
                (Post, "/applications/1/entitlements/5/consume".into()),
                (Post, "/applications/1/entitlements".into()),
                (Delete, "/applications/1/entitlements/5".into()),
                (Get, "/skus/6/subscriptions?limit=10&user_id=7".into()),
                (Get, "/skus/6/subscriptions/5".into()),
            ]
        );
        assert_eq!(
            t.body(3),
            json!({ "sku_id": "6", "owner_id": "7", "owner_type": 2 })
        );
    }
}
