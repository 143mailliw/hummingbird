use chrono::{DateTime, Utc};

use super::{
    requests::LFMRequestBuilder,
    types::{GetSession, GetToken, Session},
};

pub struct LastFMClient {
    api_key: String,
    api_secret: &'static str,
    auth_session: Option<String>,
}

impl LastFMClient {
    pub fn new(key: String, secret: &'static str) -> Self {
        LastFMClient {
            api_key: key,
            api_secret: secret,
            auth_session: None,
        }
    }

    pub fn set_session(&mut self, session: String) {
        self.auth_session = Some(session);
    }

    pub async fn get_token(&mut self) -> anyhow::Result<String> {
        let token = LFMRequestBuilder::new(self.api_key.clone())
            .add_param("method", "auth.gettoken".to_string())
            .read()
            .sign(self.api_secret)
            .send_request::<GetToken>()
            .await?;

        Ok(token.token)
    }

    pub async fn get_session(&mut self, token: String) -> anyhow::Result<Session> {
        let session = LFMRequestBuilder::new(self.api_key.clone())
            .add_param("method", "auth.getsession".to_string())
            .add_param("token", token)
            .write()
            .sign(self.api_secret)
            .send_request::<GetSession>()
            .await?;

        Ok(session.session)
    }

    pub async fn scrobble(
        &mut self,
        artist: String,
        track: String,
        timestamp: DateTime<Utc>,
        album: Option<String>,
        duration: Option<u64>,
    ) -> anyhow::Result<()> {
        let Some(session) = self.auth_session.clone() else {
            return Err(anyhow::Error::msg("not logged in"));
        };
        LFMRequestBuilder::new(self.api_key.clone())
            .add_param("method", "track.scrobble".to_string())
            .add_param("artist[0]", artist)
            .add_param("track[0]", track)
            .add_param("timestamp[0]", timestamp.timestamp().to_string())
            .add_optional_param("album[0]", album)
            .add_optional_param("duration[0]", duration.map(|a| u64::to_string(&a)))
            .add_param("sk", session)
            .write()
            .sign(self.api_secret)
            .send_write_request_ns()
            .await?;

        Ok(())
    }

    pub async fn now_playing(
        &mut self,
        artist: String,
        track: String,
        album: Option<String>,
        duration: Option<u64>,
    ) -> anyhow::Result<()> {
        let Some(session) = self.auth_session.clone() else {
            return Err(anyhow::Error::msg("not logged in"));
        };
        LFMRequestBuilder::new(self.api_key.clone())
            .add_param("method", "track.updateNowPlaying".to_string())
            .add_param("artist", artist)
            .add_param("track", track)
            .add_optional_param("album", album)
            .add_optional_param("duration", duration.map(|a| u64::to_string(&a)))
            .add_param("sk", session)
            .write()
            .sign(self.api_secret)
            .send_write_request_ns()
            .await?;

        Ok(())
    }
}
