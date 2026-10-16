//! [Auto Moderation](https://discord.com/developers/docs/resources/auto-moderation#auto-moderation-rule-object)
//! rules

use serde::Serialize;

use crate::types::{
    AutoModerationAction, AutoModerationRule, AutoModerationTriggerType, Snowflake, TriggerMetadata,
};

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    LIST_AUTOMOD_RULES = Get "/guilds/{guild_id}/auto-moderation/rules";
    GET_AUTOMOD_RULE = Get "/guilds/{guild_id}/auto-moderation/rules/{rule_id}";
    CREATE_AUTOMOD_RULE = Post "/guilds/{guild_id}/auto-moderation/rules";
    MODIFY_AUTOMOD_RULE = Patch "/guilds/{guild_id}/auto-moderation/rules/{rule_id}";
    DELETE_AUTOMOD_RULE = Delete "/guilds/{guild_id}/auto-moderation/rules/{rule_id}";
}

/// Body of [`BotClient::create_automod_rule`] (needs `name`, `event_type`, `trigger_type`,
/// `actions`) and [`BotClient::modify_automod_rule`] (`trigger_type` cannot change)
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct AutoModerationRuleFields<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,

    /// 1 message send, 2 member update
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_type: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_type: Option<AutoModerationTriggerType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_metadata: Option<&'a TriggerMetadata>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<&'a [AutoModerationAction]>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    /// Up to 20
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exempt_roles: Option<&'a [Snowflake]>,

    /// Up to 50
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exempt_channels: Option<&'a [Snowflake]>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

impl<T: Transport> BotClient<T> {
    /// [List Auto Moderation Rules for Guild](https://discord.com/developers/docs/resources/auto-moderation#list-auto-moderation-rules-for-guild)
    pub async fn list_automod_rules(
        &self,
        guild_id: Snowflake,
    ) -> Result<Response<Vec<AutoModerationRule>>> {
        let url = self.rest.url(LIST_AUTOMOD_RULES, &[&guild_id]);
        self.rest
            .call(LIST_AUTOMOD_RULES, url, Body::NONE, None)
            .await
    }

    /// [Get Auto Moderation Rule](https://discord.com/developers/docs/resources/auto-moderation#get-auto-moderation-rule)
    pub async fn get_automod_rule(
        &self,
        guild_id: Snowflake,
        rule_id: Snowflake,
    ) -> Result<Response<AutoModerationRule>> {
        let url = self.rest.url(GET_AUTOMOD_RULE, &[&guild_id, &rule_id]);
        self.rest
            .call(GET_AUTOMOD_RULE, url, Body::NONE, None)
            .await
    }

    /// [Create Auto Moderation Rule](https://discord.com/developers/docs/resources/auto-moderation#create-auto-moderation-rule)
    pub async fn create_automod_rule(
        &self,
        guild_id: Snowflake,
        rule: &AutoModerationRuleFields<'_>,
    ) -> Result<Response<AutoModerationRule>> {
        let url = self.rest.url(CREATE_AUTOMOD_RULE, &[&guild_id]);
        self.rest
            .call(CREATE_AUTOMOD_RULE, url, Body::Json(rule), rule.reason)
            .await
    }

    /// [Modify Auto Moderation Rule](https://discord.com/developers/docs/resources/auto-moderation#modify-auto-moderation-rule)
    pub async fn modify_automod_rule(
        &self,
        guild_id: Snowflake,
        rule_id: Snowflake,
        rule: &AutoModerationRuleFields<'_>,
    ) -> Result<Response<AutoModerationRule>> {
        let url = self.rest.url(MODIFY_AUTOMOD_RULE, &[&guild_id, &rule_id]);
        self.rest
            .call(MODIFY_AUTOMOD_RULE, url, Body::Json(rule), rule.reason)
            .await
    }

    /// [Delete Auto Moderation Rule](https://discord.com/developers/docs/resources/auto-moderation#delete-auto-moderation-rule)
    pub async fn delete_automod_rule(
        &self,
        guild_id: Snowflake,
        rule_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(DELETE_AUTOMOD_RULE, &[&guild_id, &rule_id]);
        self.rest
            .call_empty(DELETE_AUTOMOD_RULE, url, Body::NONE, reason)
            .await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::json;

    use super::super::{Method::*, testing::MockTransport};
    use super::*;
    use crate::types::ActionMetadata;

    #[test]
    fn routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        let (g, r) = (Snowflake::from_u64(5), Snowflake::from_u64(6));
        let metadata = TriggerMetadata {
            keyword_filter: Some(vec!["spam*".into()]),
            ..Default::default()
        };
        let actions = [AutoModerationAction {
            kind: 1,
            metadata: Some(ActionMetadata {
                custom_message: Some("no".into()),
                ..Default::default()
            }),
        }];
        let rule = AutoModerationRuleFields {
            name: Some("No spam"),
            event_type: Some(1),
            trigger_type: Some(AutoModerationTriggerType::Keyword),
            trigger_metadata: Some(&metadata),
            actions: Some(&actions),
            enabled: Some(true),
            ..Default::default()
        };
        block_on(c.list_automod_rules(g)).unwrap();
        block_on(c.get_automod_rule(g, r)).unwrap();
        block_on(c.create_automod_rule(g, &rule)).unwrap();
        block_on(c.modify_automod_rule(
            g,
            r,
            &AutoModerationRuleFields {
                enabled: Some(false),
                ..Default::default()
            },
        ))
        .unwrap();
        block_on(c.delete_automod_rule(g, r, None)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Get, "/guilds/5/auto-moderation/rules".into()),
                (Get, "/guilds/5/auto-moderation/rules/6".into()),
                (Post, "/guilds/5/auto-moderation/rules".into()),
                (Patch, "/guilds/5/auto-moderation/rules/6".into()),
                (Delete, "/guilds/5/auto-moderation/rules/6".into()),
            ]
        );
        assert_eq!(
            t.body(2),
            json!({
                "name": "No spam", "event_type": 1, "trigger_type": 1,
                "trigger_metadata": { "keyword_filter": ["spam*"] },
                "actions": [{ "type": 1, "metadata": { "custom_message": "no" } }],
                "enabled": true,
            })
        );
    }
}
