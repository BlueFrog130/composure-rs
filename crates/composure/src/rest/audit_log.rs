//! [Audit Log](https://discord.com/developers/docs/resources/audit-log#get-guild-audit-log)

use crate::types::{AuditLog, Snowflake};

use super::{
    Body, BotClient, Response, Result, Transport,
    route::{Query, routes},
};

routes! {
    GET_AUDIT_LOG = Get "/guilds/{guild_id}/audit-logs";
}

/// Filters and paging of [`BotClient::get_audit_log`], by entry id
#[derive(Debug, Clone, Copy, Default)]
pub struct GetAuditLog {
    /// Entries made by this user
    pub user_id: Option<Snowflake>,
    /// [Audit log event](https://discord.com/developers/docs/resources/audit-log#audit-log-entry-object-audit-log-events)
    pub action_type: Option<u16>,
    pub before: Option<Snowflake>,
    pub after: Option<Snowflake>,
    /// 1-100, default 50
    pub limit: Option<u8>,
}

impl<T: Transport> BotClient<T> {
    /// [Get Guild Audit Log](https://discord.com/developers/docs/resources/audit-log#get-guild-audit-log)
    pub async fn get_audit_log(
        &self,
        guild_id: Snowflake,
        query: &GetAuditLog,
    ) -> Result<Response<AuditLog>> {
        let mut url = self.rest.url(GET_AUDIT_LOG, &[&guild_id]);
        Query::new(&mut url)
            .push_opt("user_id", query.user_id)
            .push_opt("action_type", query.action_type)
            .push_opt("before", query.before)
            .push_opt("after", query.after)
            .push_opt("limit", query.limit);
        self.rest.call(GET_AUDIT_LOG, url, Body::NONE, None).await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use super::super::{Method::*, testing::MockTransport};
    use super::*;

    #[test]
    fn routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        block_on(c.get_audit_log(
            Snowflake::from_u64(5),
            &GetAuditLog {
                action_type: Some(22),
                limit: Some(10),
                ..Default::default()
            },
        ))
        .unwrap();
        assert_eq!(
            c.transport().sent(),
            [(Get, "/guilds/5/audit-logs?action_type=22&limit=10".into())]
        );
    }
}
