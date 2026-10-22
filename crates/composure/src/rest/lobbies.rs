//! [Lobbies](https://discord.com/developers/docs/resources/lobby): created and run by the app,
//! for the Social SDK; their messages also arrive as Webhook Events

use std::collections::HashMap;

use serde::Serialize;

use crate::types::{Lobby, LobbyGuildInvite, LobbyMember, LobbyMessage, Snowflake};

use super::{
    Body, BotClient, Response, Result, Transport,
    route::{Query, routes},
};

routes! {
    CREATE_LOBBY = Post "/lobbies";
    CREATE_OR_JOIN_LOBBY = Put "/lobbies";
    GET_LOBBY = Get "/lobbies/{lobby_id}";
    MODIFY_LOBBY = Patch "/lobbies/{lobby_id}";
    DELETE_LOBBY = Delete "/lobbies/{lobby_id}";
    LINK_LOBBY_CHANNEL = Patch "/lobbies/{lobby_id}/channel-linking";
    ADD_LOBBY_MEMBER = Put "/lobbies/{lobby_id}/members/{user_id}";
    REMOVE_LOBBY_MEMBER = Delete "/lobbies/{lobby_id}/members/{user_id}";
    BULK_UPDATE_LOBBY_MEMBERS = Post "/lobbies/{lobby_id}/members/bulk";
    LEAVE_LOBBY = Delete "/lobbies/{lobby_id}/members/@me";
    CREATE_LOBBY_GUILD_INVITE = Post "/lobbies/{lobby_id}/members/{user_id}/invites";
    CREATE_LOBBY_GUILD_INVITE_FOR_SELF = Post "/lobbies/{lobby_id}/members/@me/invites";
    GET_LOBBY_MESSAGES = Get "/lobbies/{lobby_id}/messages";
    SEND_LOBBY_MESSAGE = Post "/lobbies/{lobby_id}/messages";
    UPDATE_LOBBY_MESSAGE_MODERATION_METADATA = Put "/lobbies/{lobby_id}/messages/{message_id}/moderation-metadata";
}

/// Body of [`BotClient::create_lobby`] and [`BotClient::modify_lobby`]. Absent fields are
/// unchanged; `Some(None)` clears.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct LobbyFields<'a> {
    /// Seconds without anyone connected before the lobby is closed, 5-604800 (default 300)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_seconds: Option<u32>,

    /// The members (at most 25); on modify, replaces them all
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<&'a [LobbyMemberFields<'a>]>,

    /// Key-value data on the lobby (at most 25 entries)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Option<&'a HashMap<String, String>>>,

    /// [Lobby flags](https://discord.com/developers/docs/resources/lobby#lobby-object-lobby-flags)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,

    /// Where the lobby's Webhook Events go instead of the app's events URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub override_event_webhooks_url: Option<Option<&'a str>>,
}

/// A member in [`LobbyFields::members`], [`BotClient::add_lobby_member`] and
/// [`BotClient::bulk_update_lobby_members`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct LobbyMemberFields<'a> {
    /// The user
    pub id: Snowflake,

    /// Key-value data on the member (at most 25 entries)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<&'a HashMap<String, String>>,

    /// [Lobby member flags](https://discord.com/developers/docs/resources/lobby#lobby-member-object-lobby-member-flags):
    /// 1 = may link the lobby to a channel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,

    /// A name shown alongside theirs in the lobby, 1-80 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_name: Option<&'a str>,

    /// [`BotClient::bulk_update_lobby_members`] only: remove the member instead
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_member: Option<bool>,
}

/// [`LobbyMemberFields`] without the id, which [`BotClient::add_lobby_member`] puts in the path
#[derive(Serialize)]
struct AddLobbyMember<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<&'a HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    additional_name: Option<&'a str>,
}

/// Body of [`BotClient::create_or_join_lobby`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CreateOrJoinLobby<'a> {
    /// Identifies the lobby: the same secret joins the same lobby, up to 250 characters
    pub secret: &'a str,

    /// Seconds without anyone connected before the lobby is closed, 5-604800
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_timeout_seconds: Option<u32>,

    /// Key-value data on the lobby, if it is created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lobby_metadata: Option<&'a HashMap<String, String>>,

    /// Key-value data on the joining member
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_metadata: Option<&'a HashMap<String, String>>,

    /// [Lobby flags](https://discord.com/developers/docs/resources/lobby#lobby-object-lobby-flags)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u32>,
}

#[derive(Serialize)]
struct ChannelLink {
    channel_id: Option<Snowflake>,
}

#[derive(Serialize)]
struct LobbyMessageBody<'a> {
    content: &'a str,
}

impl<T: Transport> BotClient<T> {
    /// [Create Lobby](https://discord.com/developers/docs/resources/lobby#create-lobby)
    pub async fn create_lobby(&self, lobby: &LobbyFields<'_>) -> Result<Response<Lobby>> {
        let url = self.rest.url(CREATE_LOBBY, &[]);
        self.rest
            .call(CREATE_LOBBY, url, Body::Json(lobby), None)
            .await
    }

    /// [Create or Join Lobby](https://discord.com/developers/docs/resources/lobby#create-or-join-lobby):
    /// the lobby `secret` names, created if needed
    pub async fn create_or_join_lobby(
        &self,
        lobby: &CreateOrJoinLobby<'_>,
    ) -> Result<Response<Lobby>> {
        let url = self.rest.url(CREATE_OR_JOIN_LOBBY, &[]);
        self.rest
            .call(CREATE_OR_JOIN_LOBBY, url, Body::Json(lobby), None)
            .await
    }

    /// [Get Lobby](https://discord.com/developers/docs/resources/lobby#get-lobby)
    pub async fn get_lobby(&self, lobby_id: Snowflake) -> Result<Response<Lobby>> {
        let url = self.rest.url(GET_LOBBY, &[&lobby_id]);
        self.rest.call(GET_LOBBY, url, Body::NONE, None).await
    }

    /// [Modify Lobby](https://discord.com/developers/docs/resources/lobby#modify-lobby)
    pub async fn modify_lobby(
        &self,
        lobby_id: Snowflake,
        lobby: &LobbyFields<'_>,
    ) -> Result<Response<Lobby>> {
        let url = self.rest.url(MODIFY_LOBBY, &[&lobby_id]);
        self.rest
            .call(MODIFY_LOBBY, url, Body::Json(lobby), None)
            .await
    }

    /// [Delete Lobby](https://discord.com/developers/docs/resources/lobby#delete-lobby)
    pub async fn delete_lobby(&self, lobby_id: Snowflake) -> Result<()> {
        let url = self.rest.url(DELETE_LOBBY, &[&lobby_id]);
        self.rest
            .call_empty(DELETE_LOBBY, url, Body::NONE, None)
            .await
    }

    /// [Link Channel to Lobby](https://discord.com/developers/docs/resources/lobby#link-channel-to-lobby):
    /// mirrors the lobby's messages in a guild text channel; `None` unlinks
    pub async fn link_lobby_channel(
        &self,
        lobby_id: Snowflake,
        channel_id: Option<Snowflake>,
    ) -> Result<Response<Lobby>> {
        let url = self.rest.url(LINK_LOBBY_CHANNEL, &[&lobby_id]);
        self.rest
            .call(
                LINK_LOBBY_CHANNEL,
                url,
                Body::Json(&ChannelLink { channel_id }),
                None,
            )
            .await
    }

    /// [Add a Member to a Lobby](https://discord.com/developers/docs/resources/lobby#add-a-member-to-a-lobby),
    /// or update one already in it
    pub async fn add_lobby_member(
        &self,
        lobby_id: Snowflake,
        member: &LobbyMemberFields<'_>,
    ) -> Result<Response<LobbyMember>> {
        let url = self.rest.url(ADD_LOBBY_MEMBER, &[&lobby_id, &member.id]);
        let body = AddLobbyMember {
            metadata: member.metadata,
            flags: member.flags,
            additional_name: member.additional_name,
        };
        self.rest
            .call(ADD_LOBBY_MEMBER, url, Body::Json(&body), None)
            .await
    }

    /// [Remove a Member from a Lobby](https://discord.com/developers/docs/resources/lobby#remove-a-member-from-a-lobby)
    pub async fn remove_lobby_member(&self, lobby_id: Snowflake, user_id: Snowflake) -> Result<()> {
        let url = self.rest.url(REMOVE_LOBBY_MEMBER, &[&lobby_id, &user_id]);
        self.rest
            .call_empty(REMOVE_LOBBY_MEMBER, url, Body::NONE, None)
            .await
    }

    /// [Bulk Update Lobby Members](https://discord.com/developers/docs/resources/lobby#bulk-update-lobby-members):
    /// adds, updates and (with `remove_member`) removes up to 25 members at once
    pub async fn bulk_update_lobby_members(
        &self,
        lobby_id: Snowflake,
        members: &[LobbyMemberFields<'_>],
    ) -> Result<Response<Vec<LobbyMember>>> {
        let url = self.rest.url(BULK_UPDATE_LOBBY_MEMBERS, &[&lobby_id]);
        self.rest
            .call(BULK_UPDATE_LOBBY_MEMBERS, url, Body::Json(&members), None)
            .await
    }

    /// [Leave Lobby](https://discord.com/developers/docs/resources/lobby#leave-lobby)
    pub async fn leave_lobby(&self, lobby_id: Snowflake) -> Result<()> {
        let url = self.rest.url(LEAVE_LOBBY, &[&lobby_id]);
        self.rest
            .call_empty(LEAVE_LOBBY, url, Body::NONE, None)
            .await
    }

    /// [Create Linked Lobby Guild Invite for User](https://discord.com/developers/docs/resources/lobby#create-linked-lobby-guild-invite-for-user):
    /// an invite to the guild of the lobby's linked channel, for `user_id`
    pub async fn create_lobby_guild_invite(
        &self,
        lobby_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Response<LobbyGuildInvite>> {
        let url = self
            .rest
            .url(CREATE_LOBBY_GUILD_INVITE, &[&lobby_id, &user_id]);
        self.rest
            .call(CREATE_LOBBY_GUILD_INVITE, url, Body::NONE, None)
            .await
    }

    /// [Create Linked Lobby Guild Invite for Self](https://discord.com/developers/docs/resources/lobby#create-linked-lobby-guild-invite-for-self)
    pub async fn create_lobby_guild_invite_for_self(
        &self,
        lobby_id: Snowflake,
    ) -> Result<Response<LobbyGuildInvite>> {
        let url = self
            .rest
            .url(CREATE_LOBBY_GUILD_INVITE_FOR_SELF, &[&lobby_id]);
        self.rest
            .call(CREATE_LOBBY_GUILD_INVITE_FOR_SELF, url, Body::NONE, None)
            .await
    }

    /// [Get Lobby Messages](https://discord.com/developers/docs/resources/lobby#get-lobby-messages):
    /// the newest first, `limit` 1-200 (default 50)
    pub async fn get_lobby_messages(
        &self,
        lobby_id: Snowflake,
        limit: Option<u8>,
    ) -> Result<Response<Vec<LobbyMessage>>> {
        let mut url = self.rest.url(GET_LOBBY_MESSAGES, &[&lobby_id]);
        Query::new(&mut url).push_opt("limit", limit);
        self.rest
            .call(GET_LOBBY_MESSAGES, url, Body::NONE, None)
            .await
    }

    /// [Send Lobby Message](https://discord.com/developers/docs/resources/lobby#send-lobby-message)
    /// as the app, up to 4000 characters
    pub async fn send_lobby_message(
        &self,
        lobby_id: Snowflake,
        content: &str,
    ) -> Result<Response<LobbyMessage>> {
        let url = self.rest.url(SEND_LOBBY_MESSAGE, &[&lobby_id]);
        self.rest
            .call(
                SEND_LOBBY_MESSAGE,
                url,
                Body::Json(&LobbyMessageBody { content }),
                None,
            )
            .await
    }

    /// [Update Lobby Message Moderation Metadata](https://discord.com/developers/docs/resources/lobby#update-lobby-message-moderation-metadata):
    /// the app's own moderation verdict on a message, up to 5 entries
    pub async fn update_lobby_message_moderation_metadata(
        &self,
        lobby_id: Snowflake,
        message_id: Snowflake,
        metadata: &HashMap<String, String>,
    ) -> Result<()> {
        let url = self.rest.url(
            UPDATE_LOBBY_MESSAGE_MODERATION_METADATA,
            &[&lobby_id, &message_id],
        );
        self.rest
            .call_empty(
                UPDATE_LOBBY_MESSAGE_MODERATION_METADATA,
                url,
                Body::Json(metadata),
                None,
            )
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
        let (lobby, user, message) = (
            Snowflake::from_u64(5),
            Snowflake::from_u64(6),
            Snowflake::from_u64(7),
        );
        let metadata = HashMap::from([("map".to_string(), "dust".to_string())]);
        let members = [LobbyMemberFields {
            id: user,
            additional_name: Some("Captain"),
            ..Default::default()
        }];
        let create = LobbyFields {
            members: Some(&members),
            metadata: Some(Some(&metadata)),
            ..Default::default()
        };
        let removed = [LobbyMemberFields {
            id: user,
            remove_member: Some(true),
            ..Default::default()
        }];
        let join = CreateOrJoinLobby {
            secret: "game-42",
            ..Default::default()
        };
        block_on(c.create_lobby(&create)).unwrap();
        block_on(c.create_or_join_lobby(&join)).unwrap();
        block_on(c.get_lobby(lobby)).unwrap();
        block_on(c.modify_lobby(lobby, &LobbyFields::default())).unwrap();
        block_on(c.delete_lobby(lobby)).unwrap();
        block_on(c.link_lobby_channel(lobby, None)).unwrap();
        block_on(c.add_lobby_member(lobby, &members[0])).unwrap();
        block_on(c.remove_lobby_member(lobby, user)).unwrap();
        block_on(c.bulk_update_lobby_members(lobby, &removed)).unwrap();
        block_on(c.leave_lobby(lobby)).unwrap();
        block_on(c.create_lobby_guild_invite(lobby, user)).unwrap();
        block_on(c.create_lobby_guild_invite_for_self(lobby)).unwrap();
        block_on(c.get_lobby_messages(lobby, Some(10))).unwrap();
        block_on(c.send_lobby_message(lobby, "gg")).unwrap();
        block_on(c.update_lobby_message_moderation_metadata(lobby, message, &metadata)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Post, "/lobbies".into()),
                (Put, "/lobbies".into()),
                (Get, "/lobbies/5".into()),
                (Patch, "/lobbies/5".into()),
                (Delete, "/lobbies/5".into()),
                (Patch, "/lobbies/5/channel-linking".into()),
                (Put, "/lobbies/5/members/6".into()),
                (Delete, "/lobbies/5/members/6".into()),
                (Post, "/lobbies/5/members/bulk".into()),
                (Delete, "/lobbies/5/members/@me".into()),
                (Post, "/lobbies/5/members/6/invites".into()),
                (Post, "/lobbies/5/members/@me/invites".into()),
                (Get, "/lobbies/5/messages?limit=10".into()),
                (Post, "/lobbies/5/messages".into()),
                (Put, "/lobbies/5/messages/7/moderation-metadata".into()),
            ]
        );
        assert_eq!(
            t.body(0),
            json!({ "members": [{ "id": "6", "additional_name": "Captain" }], "metadata": { "map": "dust" } })
        );
        assert_eq!(t.body(1), json!({ "secret": "game-42" }));
        assert_eq!(t.body(5), json!({ "channel_id": null }), "unlinks");
        assert_eq!(
            t.body(6),
            json!({ "additional_name": "Captain" }),
            "the id is in the path"
        );
        assert_eq!(t.body(8), json!([{ "id": "6", "remove_member": true }]));
        assert_eq!(t.body(13), json!({ "content": "gg" }));
        assert_eq!(t.body(14), json!({ "map": "dust" }));
    }
}
