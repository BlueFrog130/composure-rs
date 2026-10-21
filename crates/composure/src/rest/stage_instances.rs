//! [Stage Instances](https://discord.com/developers/docs/resources/stage-instance#create-stage-instance)

use serde::Serialize;

use crate::types::{Snowflake, StageInstance, StagePrivacyLevel};

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    CREATE_STAGE_INSTANCE = Post "/stage-instances";
    GET_STAGE_INSTANCE = Get "/stage-instances/{channel_id}";
    MODIFY_STAGE_INSTANCE = Patch "/stage-instances/{channel_id}";
    DELETE_STAGE_INSTANCE = Delete "/stage-instances/{channel_id}";
}

/// Body of [`BotClient::create_stage_instance`]
#[derive(Debug, Clone, Copy, Serialize)]
pub struct CreateStageInstance<'a> {
    /// The stage channel to go live in
    pub channel_id: Snowflake,

    /// What it is about, 1-120 characters
    pub topic: &'a str,

    /// Who can see it; guild members by default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<StagePrivacyLevel>,

    /// Notify `@everyone` that the stage is live (needs the Mention Everyone permission)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send_start_notification: Option<bool>,

    /// The scheduled event it is for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_scheduled_event_id: Option<Snowflake>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Body of [`BotClient::modify_stage_instance`]. Absent fields are unchanged.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct ModifyStageInstance<'a> {
    /// What it is about, 1-120 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<&'a str>,

    /// Who can see it
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_level: Option<StagePrivacyLevel>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

impl<T: Transport> BotClient<T> {
    /// [Create Stage Instance](https://discord.com/developers/docs/resources/stage-instance#create-stage-instance):
    /// goes live; the bot must be a stage moderator
    pub async fn create_stage_instance(
        &self,
        stage: &CreateStageInstance<'_>,
    ) -> Result<Response<StageInstance>> {
        let url = self.rest.url(CREATE_STAGE_INSTANCE, &[]);
        self.rest
            .call(CREATE_STAGE_INSTANCE, url, Body::Json(stage), stage.reason)
            .await
    }

    /// [Get Stage Instance](https://discord.com/developers/docs/resources/stage-instance#get-stage-instance)
    /// of the stage channel `channel_id`
    pub async fn get_stage_instance(
        &self,
        channel_id: Snowflake,
    ) -> Result<Response<StageInstance>> {
        let url = self.rest.url(GET_STAGE_INSTANCE, &[&channel_id]);
        self.rest
            .call(GET_STAGE_INSTANCE, url, Body::NONE, None)
            .await
    }

    /// [Modify Stage Instance](https://discord.com/developers/docs/resources/stage-instance#modify-stage-instance)
    pub async fn modify_stage_instance(
        &self,
        channel_id: Snowflake,
        stage: &ModifyStageInstance<'_>,
    ) -> Result<Response<StageInstance>> {
        let url = self.rest.url(MODIFY_STAGE_INSTANCE, &[&channel_id]);
        self.rest
            .call(MODIFY_STAGE_INSTANCE, url, Body::Json(stage), stage.reason)
            .await
    }

    /// [Delete Stage Instance](https://discord.com/developers/docs/resources/stage-instance#delete-stage-instance):
    /// ends the stage
    pub async fn delete_stage_instance(
        &self,
        channel_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self.rest.url(DELETE_STAGE_INSTANCE, &[&channel_id]);
        self.rest
            .call_empty(DELETE_STAGE_INSTANCE, url, Body::NONE, reason)
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
        let channel = Snowflake::from_u64(5);
        let create = CreateStageInstance {
            channel_id: channel,
            topic: "Release party",
            privacy_level: None,
            send_start_notification: Some(true),
            guild_scheduled_event_id: None,
            reason: None,
        };
        let modify = ModifyStageInstance {
            topic: Some("Q&A"),
            ..Default::default()
        };
        block_on(c.create_stage_instance(&create)).unwrap();
        block_on(c.get_stage_instance(channel)).unwrap();
        block_on(c.modify_stage_instance(channel, &modify)).unwrap();
        block_on(c.delete_stage_instance(channel, Some("over"))).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Post, "/stage-instances".into()),
                (Get, "/stage-instances/5".into()),
                (Patch, "/stage-instances/5".into()),
                (Delete, "/stage-instances/5".into()),
            ]
        );
        assert_eq!(
            t.body(0),
            json!({ "channel_id": "5", "topic": "Release party", "send_start_notification": true })
        );
        assert_eq!(t.body(2), json!({ "topic": "Q&A" }));
        assert_eq!(t.request(3).header("x-audit-log-reason"), Some("over"));
    }
}
