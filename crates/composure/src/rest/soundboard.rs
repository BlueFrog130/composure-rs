//! [Soundboard](https://discord.com/developers/docs/resources/soundboard#endpoints)

use serde::{Deserialize, Serialize};

use crate::types::{Snowflake, SoundboardSound};

use super::{Body, BotClient, Response, Result, Transport, route::routes};

routes! {
    SEND_SOUNDBOARD_SOUND = Post "/channels/{channel_id}/send-soundboard-sound";
    LIST_DEFAULT_SOUNDBOARD_SOUNDS = Get "/soundboard-default-sounds";
    LIST_GUILD_SOUNDBOARD_SOUNDS = Get "/guilds/{guild_id}/soundboard-sounds";
    GET_GUILD_SOUNDBOARD_SOUND = Get "/guilds/{guild_id}/soundboard-sounds/{sound_id}";
    CREATE_GUILD_SOUNDBOARD_SOUND = Post "/guilds/{guild_id}/soundboard-sounds";
    MODIFY_GUILD_SOUNDBOARD_SOUND = Patch "/guilds/{guild_id}/soundboard-sounds/{sound_id}";
    DELETE_GUILD_SOUNDBOARD_SOUND = Delete "/guilds/{guild_id}/soundboard-sounds/{sound_id}";
}

/// Body of [`BotClient::create_guild_soundboard_sound`] (needs `name` and `sound`) and
/// [`BotClient::modify_guild_soundboard_sound`]
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct SoundboardSoundFields<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,

    /// Create only: MP3 or OGG data URI (max 512 KiB, 5.2 seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound: Option<&'a str>,

    /// 0-1, default 1
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<Option<f64>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<Option<Snowflake>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<Option<&'a str>>,

    /// `X-Audit-Log-Reason`
    #[serde(skip)]
    pub reason: Option<&'a str>,
}

/// Guild sounds come wrapped
#[derive(Debug, Clone, Deserialize)]
pub struct SoundboardSounds {
    pub items: Vec<SoundboardSound>,
}

#[derive(Serialize)]
struct SendSound {
    sound_id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_guild_id: Option<Snowflake>,
}

impl<T: Transport> BotClient<T> {
    /// [Send Soundboard Sound](https://discord.com/developers/docs/resources/soundboard#send-soundboard-sound)
    /// in the voice channel the bot is connected to; `source_guild_id` for another guild's sound
    pub async fn send_soundboard_sound(
        &self,
        channel_id: Snowflake,
        sound_id: Snowflake,
        source_guild_id: Option<Snowflake>,
    ) -> Result<()> {
        let url = self.rest.url(SEND_SOUNDBOARD_SOUND, &[&channel_id]);
        let body = SendSound {
            sound_id,
            source_guild_id,
        };
        self.rest
            .call_empty(SEND_SOUNDBOARD_SOUND, url, Body::Json(&body), None)
            .await
    }

    /// [List Default Soundboard Sounds](https://discord.com/developers/docs/resources/soundboard#list-default-soundboard-sounds)
    pub async fn list_default_soundboard_sounds(&self) -> Result<Response<Vec<SoundboardSound>>> {
        let url = self.rest.url(LIST_DEFAULT_SOUNDBOARD_SOUNDS, &[]);
        self.rest
            .call(LIST_DEFAULT_SOUNDBOARD_SOUNDS, url, Body::NONE, None)
            .await
    }

    /// [List Guild Soundboard Sounds](https://discord.com/developers/docs/resources/soundboard#list-guild-soundboard-sounds)
    pub async fn list_guild_soundboard_sounds(
        &self,
        guild_id: Snowflake,
    ) -> Result<Response<SoundboardSounds>> {
        let url = self.rest.url(LIST_GUILD_SOUNDBOARD_SOUNDS, &[&guild_id]);
        self.rest
            .call(LIST_GUILD_SOUNDBOARD_SOUNDS, url, Body::NONE, None)
            .await
    }

    /// [Get Guild Soundboard Sound](https://discord.com/developers/docs/resources/soundboard#get-guild-soundboard-sound)
    pub async fn get_guild_soundboard_sound(
        &self,
        guild_id: Snowflake,
        sound_id: Snowflake,
    ) -> Result<Response<SoundboardSound>> {
        let url = self
            .rest
            .url(GET_GUILD_SOUNDBOARD_SOUND, &[&guild_id, &sound_id]);
        self.rest
            .call(GET_GUILD_SOUNDBOARD_SOUND, url, Body::NONE, None)
            .await
    }

    /// [Create Guild Soundboard Sound](https://discord.com/developers/docs/resources/soundboard#create-guild-soundboard-sound)
    pub async fn create_guild_soundboard_sound(
        &self,
        guild_id: Snowflake,
        sound: &SoundboardSoundFields<'_>,
    ) -> Result<Response<SoundboardSound>> {
        let url = self.rest.url(CREATE_GUILD_SOUNDBOARD_SOUND, &[&guild_id]);
        self.rest
            .call(
                CREATE_GUILD_SOUNDBOARD_SOUND,
                url,
                Body::Json(sound),
                sound.reason,
            )
            .await
    }

    /// [Modify Guild Soundboard Sound](https://discord.com/developers/docs/resources/soundboard#modify-guild-soundboard-sound)
    pub async fn modify_guild_soundboard_sound(
        &self,
        guild_id: Snowflake,
        sound_id: Snowflake,
        sound: &SoundboardSoundFields<'_>,
    ) -> Result<Response<SoundboardSound>> {
        let url = self
            .rest
            .url(MODIFY_GUILD_SOUNDBOARD_SOUND, &[&guild_id, &sound_id]);
        let sound = SoundboardSoundFields {
            sound: None,
            ..*sound
        };
        self.rest
            .call(
                MODIFY_GUILD_SOUNDBOARD_SOUND,
                url,
                Body::Json(&sound),
                sound.reason,
            )
            .await
    }

    /// [Delete Guild Soundboard Sound](https://discord.com/developers/docs/resources/soundboard#delete-guild-soundboard-sound)
    pub async fn delete_guild_soundboard_sound(
        &self,
        guild_id: Snowflake,
        sound_id: Snowflake,
        reason: Option<&str>,
    ) -> Result<()> {
        let url = self
            .rest
            .url(DELETE_GUILD_SOUNDBOARD_SOUND, &[&guild_id, &sound_id]);
        self.rest
            .call_empty(DELETE_GUILD_SOUNDBOARD_SOUND, url, Body::NONE, reason)
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
        let (g, s) = (Snowflake::from_u64(5), Snowflake::from_u64(6));
        let create = SoundboardSoundFields {
            name: Some("boop"),
            sound: Some("data:audio/ogg;base64,AA=="),
            volume: Some(Some(0.5)),
            ..Default::default()
        };
        block_on(c.send_soundboard_sound(g, s, None)).unwrap();
        block_on(c.list_default_soundboard_sounds()).unwrap();
        block_on(c.list_guild_soundboard_sounds(g)).unwrap();
        block_on(c.get_guild_soundboard_sound(g, s)).unwrap();
        block_on(c.create_guild_soundboard_sound(g, &create)).unwrap();
        block_on(c.modify_guild_soundboard_sound(g, s, &create)).unwrap();
        block_on(c.delete_guild_soundboard_sound(g, s, None)).unwrap();
        let t = c.transport();
        assert_eq!(
            t.sent(),
            [
                (Post, "/channels/5/send-soundboard-sound".into()),
                (Get, "/soundboard-default-sounds".into()),
                (Get, "/guilds/5/soundboard-sounds".into()),
                (Get, "/guilds/5/soundboard-sounds/6".into()),
                (Post, "/guilds/5/soundboard-sounds".into()),
                (Patch, "/guilds/5/soundboard-sounds/6".into()),
                (Delete, "/guilds/5/soundboard-sounds/6".into()),
            ]
        );
        assert_eq!(t.body(0), json!({ "sound_id": "6" }));
        assert_eq!(
            t.body(5),
            json!({ "name": "boop", "volume": 0.5 }),
            "the sound itself cannot change"
        );
    }
}
