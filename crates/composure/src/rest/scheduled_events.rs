//! [Guild Scheduled Events](https://discord.com/developers/docs/resources/guild-scheduled-event#endpoints)

use serde::Serialize;
use serde_json::value::RawValue;

use crate::types::{
    EntityMetadata, ScheduledEvent, ScheduledEventEntityType, ScheduledEventStatus,
    ScheduledEventUser, Snowflake,
};

use super::{
    Body, BotClient, Response, Result, Transport,
    route::{Query, routes},
};

routes! {
    LIST_SCHEDULED_EVENTS = Get "/guilds/{guild_id}/scheduled-events";
    GET_SCHEDULED_EVENT = Get "/guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}";
    CREATE_SCHEDULED_EVENT = Post "/guilds/{guild_id}/scheduled-events";
    MODIFY_SCHEDULED_EVENT = Patch "/guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}";
    DELETE_SCHEDULED_EVENT = Delete "/guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}";
    GET_SCHEDULED_EVENT_USERS = Get "/guilds/{guild_id}/scheduled-events/{guild_scheduled_event_id}/users";
}

/// Body of [`BotClient::create_scheduled_event`] and [`BotClient::modify_scheduled_event`].
/// Creating needs `name`, `scheduled_start_time`, `entity_type`, and a `channel_id` (stage and
/// voice) or `entity_metadata.location` plus `scheduled_end_time` (external). Times are ISO 8601.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ScheduledEventFields<'a> {
    /// 1-100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,

    /// 1-1000 characters; `Some(None)` removes it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<&'a str>>,

    /// When it starts (ISO 8601)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_start_time: Option<&'a str>,

    /// When it ends (ISO 8601); required for external events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_end_time: Option<&'a str>,

    /// 2 (guild only) is the only level; defaulted when creating
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<u8>,

    /// Stage, voice, or external
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_type: Option<ScheduledEventEntityType>,

    /// `Some(None)` when changing to an external event
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Option<Snowflake>>,

    /// External events: where
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_metadata: Option<&'a EntityMetadata>,

    /// Modify only: start, end, or cancel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ScheduledEventStatus>,

    /// Cover image as a data URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<&'a str>,

    /// [Recurrence rule](https://discord.com/developers/docs/resources/guild-scheduled-event#guild-scheduled-event-recurrence-rule-object)
    /// JSON; `Some(None)` removes it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<Option<&'a RawValue>>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Paging of [`BotClient::get_scheduled_event_users`], by user id
#[derive(Debug, Clone, Copy, Default)]
pub struct GetScheduledEventUsers {
    /// 1-100, default 100
    pub limit: Option<u8>,
    /// Include each user's guild member
    pub with_member: bool,
    /// Users before this id
    pub before: Option<Snowflake>,
    /// Users after this id
    pub after: Option<Snowflake>,
}

impl<T: Transport> BotClient<T> {
    /// [List Scheduled Events for Guild](https://discord.com/developers/docs/resources/guild-scheduled-event#list-scheduled-events-for-guild)
    pub async fn list_scheduled_events(
        &self,
        guild_id: Snowflake,
        with_user_count: bool,
    ) -> Result<Response<Vec<ScheduledEvent>>> {
        let mut url = self.rest.url(LIST_SCHEDULED_EVENTS, &[&guild_id]);
        Query::new(&mut url).push_opt("with_user_count", with_user_count.then_some(true));
        self.rest
            .call(LIST_SCHEDULED_EVENTS, url, Body::NONE, None)
            .await
    }

    /// [Get Guild Scheduled Event](https://discord.com/developers/docs/resources/guild-scheduled-event#get-guild-scheduled-event)
    pub async fn get_scheduled_event(
        &self,
        guild_id: Snowflake,
        event_id: Snowflake,
        with_user_count: bool,
    ) -> Result<Response<ScheduledEvent>> {
        let mut url = self.rest.url(GET_SCHEDULED_EVENT, &[&guild_id, &event_id]);
        Query::new(&mut url).push_opt("with_user_count", with_user_count.then_some(true));
        self.rest
            .call(GET_SCHEDULED_EVENT, url, Body::NONE, None)
            .await
    }

    /// [Create Guild Scheduled Event](https://discord.com/developers/docs/resources/guild-scheduled-event#create-guild-scheduled-event)
    pub async fn create_scheduled_event(
        &self,
        guild_id: Snowflake,
        event: &ScheduledEventFields<'_>,
    ) -> Result<Response<ScheduledEvent>> {
        let url = self.rest.url(CREATE_SCHEDULED_EVENT, &[&guild_id]);
        let event = ScheduledEventFields {
            privacy_level: event.privacy_level.or(Some(2)),
            ..*event
        };
        self.rest
            .call(
                CREATE_SCHEDULED_EVENT,
                url,
                Body::Json(&event),
                event.reason,
            )
            .await
    }

    /// [Modify Guild Scheduled Event](https://discord.com/developers/docs/resources/guild-scheduled-event#modify-guild-scheduled-event)
    pub async fn modify_scheduled_event(
        &self,
        guild_id: Snowflake,
        event_id: Snowflake,
        event: &ScheduledEventFields<'_>,
    ) -> Result<Response<ScheduledEvent>> {
        let url = self
            .rest
            .url(MODIFY_SCHEDULED_EVENT, &[&guild_id, &event_id]);
        self.rest
            .call(MODIFY_SCHEDULED_EVENT, url, Body::Json(event), event.reason)
            .await
    }

    /// [Delete Guild Scheduled Event](https://discord.com/developers/docs/resources/guild-scheduled-event#delete-guild-scheduled-event)
    pub async fn delete_scheduled_event(
        &self,
        guild_id: Snowflake,
        event_id: Snowflake,
    ) -> Result<()> {
        let url = self
            .rest
            .url(DELETE_SCHEDULED_EVENT, &[&guild_id, &event_id]);
        self.rest
            .call_empty(DELETE_SCHEDULED_EVENT, url, Body::NONE, None)
            .await
    }

    /// [Get Guild Scheduled Event Users](https://discord.com/developers/docs/resources/guild-scheduled-event#get-guild-scheduled-event-users):
    /// who is interested
    pub async fn get_scheduled_event_users(
        &self,
        guild_id: Snowflake,
        event_id: Snowflake,
        query: &GetScheduledEventUsers,
    ) -> Result<Response<Vec<ScheduledEventUser>>> {
        let mut url = self
            .rest
            .url(GET_SCHEDULED_EVENT_USERS, &[&guild_id, &event_id]);
        Query::new(&mut url)
            .push_opt("limit", query.limit)
            .push_opt("with_member", query.with_member.then_some(true))
            .push_opt("before", query.before)
            .push_opt("after", query.after);
        self.rest
            .call(GET_SCHEDULED_EVENT_USERS, url, Body::NONE, None)
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
        let (g, e) = (Snowflake::from_u64(5), Snowflake::from_u64(6));
        let location = EntityMetadata {
            location: Some("Park".into()),
        };
        let create = ScheduledEventFields {
            name: Some("Meetup"),
            scheduled_start_time: Some("2030-01-01T00:00:00Z"),
            scheduled_end_time: Some("2030-01-01T02:00:00Z"),
            entity_type: Some(ScheduledEventEntityType::External),
            entity_metadata: Some(&location),
            ..Default::default()
        };
        let cancel = ScheduledEventFields {
            status: Some(ScheduledEventStatus::Canceled),
            ..Default::default()
        };
        block_on(c.list_scheduled_events(g, true)).unwrap();
        block_on(c.get_scheduled_event(g, e, false)).unwrap();
        block_on(c.create_scheduled_event(g, &create)).unwrap();
        block_on(c.modify_scheduled_event(g, e, &cancel)).unwrap();
        block_on(c.delete_scheduled_event(g, e)).unwrap();
        block_on(c.get_scheduled_event_users(
            g,
            e,
            &GetScheduledEventUsers {
                with_member: true,
                ..Default::default()
            },
        ))
        .unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (
                    Get,
                    "/guilds/5/scheduled-events?with_user_count=true".into()
                ),
                (Get, "/guilds/5/scheduled-events/6".into()),
                (Post, "/guilds/5/scheduled-events".into()),
                (Patch, "/guilds/5/scheduled-events/6".into()),
                (Delete, "/guilds/5/scheduled-events/6".into()),
                (
                    Get,
                    "/guilds/5/scheduled-events/6/users?with_member=true".into()
                ),
            ]
        );
        assert_eq!(
            t.body(2),
            json!({
                "name": "Meetup", "scheduled_start_time": "2030-01-01T00:00:00Z",
                "scheduled_end_time": "2030-01-01T02:00:00Z", "privacy_level": 2,
                "entity_type": 3, "entity_metadata": { "location": "Park" },
            })
        );
        assert_eq!(t.body(3), json!({ "status": 4 }));
    }
}
