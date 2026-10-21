//! [Users](https://discord.com/developers/docs/resources/user#endpoints)

use serde::Serialize;

use crate::types::{Channel, Snowflake, User};

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    GET_CURRENT_USER = Get "/users/@me";
    GET_USER = Get "/users/{user_id}";
    CREATE_DM = Post "/users/@me/channels";
}

#[derive(Serialize)]
struct CreateDm {
    recipient_id: Snowflake,
}

impl<T: Transport> BotClient<T> {
    /// [Get Current User](https://discord.com/developers/docs/resources/user#get-current-user):
    /// the bot itself
    pub async fn get_current_user(&self) -> Result<Response<User>> {
        let url = self.rest.url(GET_CURRENT_USER, &[]);
        self.rest
            .call(GET_CURRENT_USER, url, Body::NONE, None)
            .await
    }

    /// [Get User](https://discord.com/developers/docs/resources/user#get-user)
    pub async fn get_user(&self, user_id: Snowflake) -> Result<Response<User>> {
        let url = self.rest.url(GET_USER, &[&user_id]);
        self.rest.call(GET_USER, url, Body::NONE, None).await
    }

    /// [Create DM](https://discord.com/developers/docs/resources/user#create-dm): the DM channel
    /// with `recipient_id`, to send messages to with `create_message`
    pub async fn create_dm(&self, recipient_id: Snowflake) -> Result<Response<Channel>> {
        let url = self.rest.url(CREATE_DM, &[]);
        self.rest
            .call(CREATE_DM, url, Body::Json(&CreateDm { recipient_id }), None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use serde_json::json;

    use super::super::{Method, testing::MockTransport};
    use super::*;

    #[test]
    fn routes() {
        let c = BotClient::new(MockTransport::default(), "t", Snowflake::from_u64(1));
        let user = Snowflake::from_u64(9);
        block_on(c.get_current_user()).unwrap();
        block_on(c.get_user(user)).unwrap();
        block_on(c.create_dm(user)).unwrap();
        let sent: Vec<_> = c
            .transport()
            .requests()
            .iter()
            .map(|r| (r.method, r.url.rsplit("v10").next().unwrap().to_string()))
            .collect();
        assert_eq!(
            sent,
            [
                (Method::Get, "/users/@me".into()),
                (Method::Get, "/users/9".into()),
                (Method::Post, "/users/@me/channels".into()),
            ]
        );
        assert_eq!(c.transport().body(2), json!({ "recipient_id": "9" }));
    }
}
