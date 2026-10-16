//! Guild administration objects: scheduled events, auto moderation, audit log, webhooks

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::{Channel, Member, Snowflake, User};

/// [Guild Scheduled Event](https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-object)
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduledEvent {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    /// Stage or voice channel; `None` for external events
    pub channel_id: Option<Snowflake>,
    pub creator_id: Option<Snowflake>,
    pub name: String,
    pub description: Option<String>,
    /// ISO 8601
    pub scheduled_start_time: String,
    /// ISO 8601; required for external events
    pub scheduled_end_time: Option<String>,
    /// 2: guild only
    pub privacy_level: u8,
    pub status: ScheduledEventStatus,
    pub entity_type: ScheduledEventEntityType,
    pub entity_id: Option<Snowflake>,
    pub entity_metadata: Option<EntityMetadata>,
    pub creator: Option<User>,
    /// With `with_user_count`
    pub user_count: Option<u32>,
    /// Cover image hash
    pub image: Option<String>,
    /// [Recurrence rule](https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-recurrence-rule-object), as sent
    pub recurrence_rule: Option<Box<RawValue>>,
}

/// [Event Status](https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-object-guild-scheduled-event-status)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum ScheduledEventStatus {
    Scheduled = 1,
    Active = 2,
    Completed = 3,
    Canceled = 4,
}

/// [Entity Type](https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-object-guild-scheduled-event-entity-types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum ScheduledEventEntityType {
    StageInstance = 1,
    Voice = 2,
    External = 3,
}

/// Where an external event happens
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EntityMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// [Scheduled Event User](https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-user-object)
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduledEventUser {
    pub guild_scheduled_event_id: Snowflake,
    pub user: User,
    /// With `with_member`
    pub member: Option<Member>,
}

/// [Auto Moderation Rule](https://discord.com/developers/docs/resources/auto-moderation#auto-moderation-rule-object)
#[derive(Debug, Clone, Deserialize)]
pub struct AutoModerationRule {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub name: String,
    pub creator_id: Snowflake,
    /// 1 message send, 2 member update
    pub event_type: u8,
    pub trigger_type: AutoModerationTriggerType,
    pub trigger_metadata: TriggerMetadata,
    pub actions: Vec<AutoModerationAction>,
    pub enabled: bool,
    pub exempt_roles: Vec<Snowflake>,
    pub exempt_channels: Vec<Snowflake>,
}

/// [Trigger Type](https://discord.com/developers/docs/resources/auto-moderation#auto-moderation-rule-object-trigger-types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum AutoModerationTriggerType {
    Keyword = 1,
    Spam = 3,
    KeywordPreset = 4,
    MentionSpam = 5,
    MemberProfile = 6,
}

/// [Trigger Metadata](https://discord.com/developers/docs/resources/auto-moderation#auto-moderation-rule-object-trigger-metadata);
/// which fields apply depends on the trigger type
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TriggerMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword_filter: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex_patterns: Option<Vec<String>>,

    /// 1 profanity, 2 sexual content, 3 slurs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presets: Option<Vec<u8>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_list: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention_total_limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention_raid_protection_enabled: Option<bool>,
}

/// [Auto Moderation Action](https://discord.com/developers/docs/resources/auto-moderation#auto-moderation-action-object)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AutoModerationAction {
    /// 1 block message, 2 send alert, 3 timeout, 4 block member interaction
    #[serde(rename = "type")]
    pub kind: u8,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ActionMetadata>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ActionMetadata {
    /// Send alert: where
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,

    /// Timeout: how long (max 4 weeks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u32>,

    /// Block message: shown to the author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_message: Option<String>,
}

/// [Audit Log](https://discord.com/developers/docs/resources/audit-log#audit-log-object): entries
/// plus the objects they reference
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLog {
    pub audit_log_entries: Vec<AuditLogEntry>,
    #[serde(default)]
    pub users: Vec<User>,
    #[serde(default)]
    pub threads: Vec<Channel>,
    #[serde(default)]
    pub guild_scheduled_events: Vec<ScheduledEvent>,
    #[serde(default)]
    pub auto_moderation_rules: Vec<Option<AutoModerationRule>>,
    #[serde(default)]
    pub webhooks: Vec<Webhook>,
    /// Integrations and application commands, as sent
    #[serde(default)]
    pub integrations: Vec<Box<RawValue>>,
    #[serde(default)]
    pub application_commands: Vec<Box<RawValue>>,
}

/// [Audit Log Entry](https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object)
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogEntry {
    pub id: Snowflake,
    /// [Audit log event](https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object-audit-log-events)
    pub action_type: u16,
    /// Who did it
    pub user_id: Option<Snowflake>,
    /// What it was done to
    pub target_id: Option<String>,
    #[serde(default)]
    pub changes: Vec<AuditLogChange>,
    pub options: Option<AuditLogOptions>,
    pub reason: Option<String>,
}

/// [Audit Log Change](https://discord.com/developers/docs/resources/audit-log#audit-log-change-object):
/// values are as sent, their type depends on `key`
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogChange {
    pub key: String,
    pub new_value: Option<Box<RawValue>>,
    pub old_value: Option<Box<RawValue>>,
}

/// [Optional Audit Entry Info](https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object-optional-audit-entry-info)
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuditLogOptions {
    pub application_id: Option<Snowflake>,
    pub auto_moderation_rule_name: Option<String>,
    pub auto_moderation_rule_trigger_type: Option<String>,
    pub channel_id: Option<Snowflake>,
    pub count: Option<String>,
    pub delete_member_days: Option<String>,
    pub id: Option<Snowflake>,
    pub members_removed: Option<String>,
    pub message_id: Option<Snowflake>,
    pub role_name: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub integration_type: Option<String>,
}

/// [Webhook](https://discord.com/developers/docs/resources/webhook#webhook-object)
#[derive(Debug, Clone, Deserialize)]
pub struct Webhook {
    pub id: Snowflake,
    /// 1 incoming, 2 channel follower, 3 application
    #[serde(rename = "type")]
    pub kind: u8,
    pub guild_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    /// Who created it (absent when fetched by token)
    pub user: Option<User>,
    pub name: Option<String>,
    pub avatar: Option<String>,
    /// Incoming webhooks: the secure token
    pub token: Option<String>,
    pub application_id: Option<Snowflake>,
    /// Incoming webhooks: execution URL
    pub url: Option<String>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn deserializes_admin_objects() {
        let event: ScheduledEvent = serde_json::from_value(json!({
            "id": "1", "guild_id": "2", "channel_id": null, "creator_id": "3", "name": "Meetup",
            "description": null, "scheduled_start_time": "2030-01-01T00:00:00+00:00",
            "scheduled_end_time": "2030-01-01T02:00:00+00:00", "privacy_level": 2, "status": 1,
            "entity_type": 3, "entity_id": null, "entity_metadata": { "location": "Park" },
            "image": null, "recurrence_rule": null, "guild_scheduled_event_exceptions": [],
        }))
        .unwrap();
        assert_eq!(event.entity_type, ScheduledEventEntityType::External);
        assert_eq!(
            event.entity_metadata.unwrap().location.as_deref(),
            Some("Park")
        );

        let rule: AutoModerationRule = serde_json::from_value(json!({
            "id": "1", "guild_id": "2", "creator_id": "3", "name": "No spam", "event_type": 1,
            "trigger_type": 1, "enabled": true, "exempt_roles": [], "exempt_channels": [],
            "trigger_metadata": { "keyword_filter": ["spam"], "regex_patterns": [], "allow_list": [] },
            "actions": [{ "type": 1, "metadata": { "custom_message": "no" } }],
        }))
        .unwrap();
        assert_eq!(rule.trigger_type, AutoModerationTriggerType::Keyword);

        let log: AuditLog = serde_json::from_value(json!({
            "audit_log_entries": [{
                "id": "9", "action_type": 22, "user_id": "3", "target_id": "4",
                "changes": [{ "key": "nick", "new_value": "a" }],
                "options": { "delete_member_days": "1", "members_removed": "2" },
                "reason": "spam",
            }],
            "users": [], "integrations": [], "webhooks": [], "guild_scheduled_events": [],
            "threads": [], "application_commands": [], "auto_moderation_rules": [],
        }))
        .unwrap();
        let entry = &log.audit_log_entries[0];
        assert_eq!(entry.changes[0].new_value.as_ref().unwrap().get(), "\"a\"");
        assert_eq!(
            entry.options.as_ref().unwrap().members_removed.as_deref(),
            Some("2")
        );
    }
}
