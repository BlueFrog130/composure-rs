//! Guild administration objects: scheduled events, auto moderation, audit log, webhooks

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::types::{Channel, Member, Snowflake, User};

/// [Guild Scheduled Event](https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-object)
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduledEvent {
    /// The event's id
    pub id: Snowflake,
    /// The guild it belongs to
    pub guild_id: Snowflake,
    /// Stage or voice channel; `None` for external events
    pub channel_id: Option<Snowflake>,
    /// Who created it
    pub creator_id: Option<Snowflake>,
    /// 1-100 characters
    pub name: String,
    /// 1-1000 characters
    pub description: Option<String>,
    /// ISO 8601
    pub scheduled_start_time: String,
    /// ISO 8601; required for external events
    pub scheduled_end_time: Option<String>,
    /// 2: guild only
    pub privacy_level: u8,
    /// Scheduled, active, completed or canceled
    pub status: ScheduledEventStatus,
    /// Where it happens: a stage, a voice channel, or elsewhere
    pub entity_type: ScheduledEventEntityType,
    /// The stage instance, for stage events
    pub entity_id: Option<Snowflake>,
    /// The location, for external events
    pub entity_metadata: Option<EntityMetadata>,
    /// Who created it (absent for events created before October 2021)
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
    /// Not started yet
    Scheduled = 1,
    /// Happening now
    Active = 2,
    /// Ended
    Completed = 3,
    /// Canceled before it started
    Canceled = 4,
}

/// [Entity Type](https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-object-guild-scheduled-event-entity-types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum ScheduledEventEntityType {
    /// In a stage channel
    StageInstance = 1,
    /// In a voice channel
    Voice = 2,
    /// Somewhere else, at `entity_metadata.location`
    External = 3,
}

/// Where an external event happens
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct EntityMetadata {
    /// Where it happens, 1-100 characters; required for external events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

/// [Scheduled Event User](https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-user-object)
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduledEventUser {
    /// The event
    pub guild_scheduled_event_id: Snowflake,
    /// The interested user
    pub user: User,
    /// With `with_member`
    pub member: Option<Member>,
}

/// [Auto Moderation Rule](https://discord.com/developers/docs/resources/auto-moderation#auto-moderation-rule-object)
#[derive(Debug, Clone, Deserialize)]
pub struct AutoModerationRule {
    /// The rule's id
    pub id: Snowflake,
    /// The guild it belongs to
    pub guild_id: Snowflake,
    /// The rule's name
    pub name: String,
    /// Who created it
    pub creator_id: Snowflake,
    /// 1 message send, 2 member update
    pub event_type: u8,
    /// What triggers it
    pub trigger_type: AutoModerationTriggerType,
    /// Settings of the trigger
    pub trigger_metadata: TriggerMetadata,
    /// What happens when it triggers
    pub actions: Vec<AutoModerationAction>,
    /// Whether it is on
    pub enabled: bool,
    /// Roles it does not apply to (max 20)
    pub exempt_roles: Vec<Snowflake>,
    /// Channels it does not apply to (max 50)
    pub exempt_channels: Vec<Snowflake>,
}

/// [Trigger Type](https://discord.com/developers/docs/resources/auto-moderation#auto-moderation-rule-object-trigger-types)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize_repr, Serialize_repr)]
#[repr(u8)]
pub enum AutoModerationTriggerType {
    /// Messages containing words from a list
    Keyword = 1,
    /// Generic spam
    Spam = 3,
    /// Words from Discord's predefined lists
    KeywordPreset = 4,
    /// Too many unique mentions
    MentionSpam = 5,
    /// Words in member profiles
    MemberProfile = 6,
}

/// [Trigger Metadata](https://discord.com/developers/docs/resources/auto-moderation#auto-moderation-rule-object-trigger-metadata);
/// which fields apply depends on the trigger type
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TriggerMetadata {
    /// Keyword: substrings that trigger (max 1000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyword_filter: Option<Vec<String>>,

    /// Keyword and member profile: regular expressions that trigger (max 10)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex_patterns: Option<Vec<String>>,

    /// 1 profanity, 2 sexual content, 3 slurs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presets: Option<Vec<u8>>,

    /// Keyword and keyword preset: substrings that never trigger
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_list: Option<Vec<String>>,

    /// Mention spam: unique mentions allowed per message (max 50)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention_total_limit: Option<u32>,

    /// Mention spam: whether to detect mention raids
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention_raid_protection_enabled: Option<bool>,
}

/// [Auto Moderation Action](https://discord.com/developers/docs/resources/auto-moderation#auto-moderation-action-object)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AutoModerationAction {
    /// 1 block message, 2 send alert, 3 timeout, 4 block member interaction
    #[serde(rename = "type")]
    pub kind: u8,

    /// Settings for the action type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ActionMetadata>,
}

/// Settings of an [`AutoModerationAction`], depending on its type
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
    /// The entries, newest first
    pub audit_log_entries: Vec<AuditLogEntry>,
    /// Users referenced by the entries
    #[serde(default)]
    pub users: Vec<User>,
    /// Threads referenced by the entries
    #[serde(default)]
    pub threads: Vec<Channel>,
    /// Scheduled events referenced by the entries
    #[serde(default)]
    pub guild_scheduled_events: Vec<ScheduledEvent>,
    /// Auto moderation rules referenced by the entries
    #[serde(default)]
    pub auto_moderation_rules: Vec<Option<AutoModerationRule>>,
    /// Webhooks referenced by the entries
    #[serde(default)]
    pub webhooks: Vec<Webhook>,
    /// Integrations and application commands, as sent
    #[serde(default)]
    pub integrations: Vec<Box<RawValue>>,
    /// Application Commands referenced by the entries, as sent
    #[serde(default)]
    pub application_commands: Vec<Box<RawValue>>,
}

/// [Audit Log Entry](https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object)
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogEntry {
    /// The entry's id
    pub id: Snowflake,
    /// [Audit log event](https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object-audit-log-events)
    pub action_type: u16,
    /// Who did it
    pub user_id: Option<Snowflake>,
    /// What it was done to
    pub target_id: Option<String>,
    /// What changed
    #[serde(default)]
    pub changes: Vec<AuditLogChange>,
    /// Extra details for some action types
    pub options: Option<AuditLogOptions>,
    /// Why it was done (0-512 characters)
    pub reason: Option<String>,
}

/// [Audit Log Change](https://discord.com/developers/docs/resources/audit-log#audit-log-change-object):
/// values are as sent, their type depends on `key`
#[derive(Debug, Clone, Deserialize)]
pub struct AuditLogChange {
    /// The changed field's name
    pub key: String,
    /// The value after the change
    pub new_value: Option<Box<RawValue>>,
    /// The value before the change
    pub old_value: Option<Box<RawValue>>,
}

/// [Optional Audit Entry Info](https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object-optional-audit-entry-info)
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuditLogOptions {
    /// The application whose permissions were changed
    pub application_id: Option<Snowflake>,
    /// The triggered auto moderation rule's name
    pub auto_moderation_rule_name: Option<String>,
    /// The triggered auto moderation rule's trigger type
    pub auto_moderation_rule_trigger_type: Option<String>,
    /// The channel in which entities were targeted
    pub channel_id: Option<Snowflake>,
    /// How many entities were targeted
    pub count: Option<String>,
    /// Days after which inactive members were kicked
    pub delete_member_days: Option<String>,
    /// The id of the overwritten entity
    pub id: Option<Snowflake>,
    /// How many members were removed by the prune
    pub members_removed: Option<String>,
    /// The id of the message that was targeted
    pub message_id: Option<Snowflake>,
    /// The name of the role, for overwrite entries
    pub role_name: Option<String>,
    /// The overwritten entity's type: `"0"` role, `"1"` member
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// The type of integration which performed the action
    pub integration_type: Option<String>,
}

/// [Webhook](https://discord.com/developers/docs/resources/webhook#webhook-object)
#[derive(Debug, Clone, Deserialize)]
pub struct Webhook {
    /// The webhook's id
    pub id: Snowflake,
    /// 1 incoming, 2 channel follower, 3 application
    #[serde(rename = "type")]
    pub kind: u8,
    /// The guild it belongs to
    pub guild_id: Option<Snowflake>,
    /// The channel it posts to
    pub channel_id: Option<Snowflake>,
    /// Who created it (absent when fetched by token)
    pub user: Option<User>,
    /// Default name (1-80 characters)
    pub name: Option<String>,
    /// Default avatar hash
    pub avatar: Option<String>,
    /// Incoming webhooks: the secure token
    pub token: Option<String>,
    /// The application that created it
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
