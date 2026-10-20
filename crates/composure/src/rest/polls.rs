//! [Polls](https://discord.com/developers/docs/resources/poll#get-answer-voters): voters and
//! ending early. Polls are created with a message's `poll` field.

use crate::types::{Message, PollAnswerVoters, Snowflake};

use super::{
    Body, BotClient, Response, Result, Transport,
    route::{Query, routes},
};

routes! {
    GET_ANSWER_VOTERS = Get "/channels/{channel_id}/polls/{message_id}/answers/{answer_id}";
    END_POLL = Post "/channels/{channel_id}/polls/{message_id}/expire";
}

impl<T: Transport> BotClient<T> {
    /// [Get Answer Voters](https://discord.com/developers/docs/resources/poll#get-answer-voters),
    /// paged by user id (1-100, default 25)
    pub async fn get_answer_voters(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
        answer_id: u32,
        after: Option<Snowflake>,
        limit: Option<u8>,
    ) -> Result<Response<PollAnswerVoters>> {
        let mut url = self
            .rest
            .url(GET_ANSWER_VOTERS, &[&channel_id, &message_id, &answer_id]);
        Query::new(&mut url)
            .push_opt("after", after)
            .push_opt("limit", limit);
        self.rest
            .call(GET_ANSWER_VOTERS, url, Body::NONE, None)
            .await
    }

    /// [End Poll](https://discord.com/developers/docs/resources/poll#end-poll) the bot created
    pub async fn end_poll(
        &self,
        channel_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<Response<Message>> {
        let url = self.rest.url(END_POLL, &[&channel_id, &message_id]);
        self.rest.call(END_POLL, url, Body::NONE, None).await
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
        let (ch, m) = (Snowflake::from_u64(5), Snowflake::from_u64(6));
        block_on(c.get_answer_voters(ch, m, 2, None, Some(50))).unwrap();
        block_on(c.end_poll(ch, m)).unwrap();
        assert_eq!(
            c.transport().sent(),
            [
                (Get, "/channels/5/polls/6/answers/2?limit=50".into()),
                (Post, "/channels/5/polls/6/expire".into()),
            ]
        );
    }
}
