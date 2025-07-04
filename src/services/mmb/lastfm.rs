use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use client::LastFMClient;
use smol::block_on;
use tracing::{debug, warn};

use crate::{media::metadata::Metadata, playback::thread::PlaybackState};

use super::MediaMetadataBroadcastService;

pub mod client;
mod requests;
pub mod types;
mod util;

pub const LASTFM_API_KEY: Option<&'static str> = option_env!("LASTFM_API_KEY");
pub const LASTFM_API_SECRET: Option<&'static str> = option_env!("LASTFM_API_SECRET");

pub struct LastFM {
    client: LastFMClient,
    start_timestamp: Option<DateTime<Utc>>,
    accumulated_time: u64,
    duration: u64,
    metadata: Option<Arc<Metadata>>,
    last_postion: u64,
    should_scrobble: bool,
}

impl LastFM {
    pub fn new(client: LastFMClient) -> Self {
        LastFM {
            client,
            start_timestamp: None,
            accumulated_time: 0,
            metadata: None,
            duration: 0,
            last_postion: 0,
            should_scrobble: false,
        }
    }

    pub async fn scrobble(&mut self) {
        let Some(info) = &self.metadata else {
            return;
        };
        let (Some(artist), Some(track)) = (info.artist.clone(), info.name.clone()) else {
            return;
        };
        if let Err(e) = self
            .client
            .scrobble(
                artist,
                track,
                self.start_timestamp.unwrap(),
                info.album.clone(),
                None,
            )
            .await
        {
            warn!("Could not scrobble: {}", e)
        }
    }
}

#[async_trait]
impl MediaMetadataBroadcastService for LastFM {
    async fn new_track(&mut self, _: PathBuf) {
        if self.should_scrobble {
            debug!("attempting scrobble");
            self.scrobble().await;
        }

        self.start_timestamp = Some(chrono::offset::Utc::now());
        self.accumulated_time = 0;
        self.last_postion = 0;
        self.should_scrobble = false;
    }

    async fn metadata_recieved(&mut self, info: Arc<Metadata>) {
        let (Some(artist), Some(track)) = (info.artist.clone(), info.name.clone()) else {
            return;
        };
        if let Err(e) = self
            .client
            .now_playing(artist, track, info.album.clone(), None)
            .await
        {
            warn!("Could not set now playing: {}", e)
        }

        self.metadata = Some(info);
    }

    async fn state_changed(&mut self, state: PlaybackState) {
        if self.should_scrobble && state != PlaybackState::Playing {
            debug!("attempting scrobble");
            self.scrobble().await;
            self.should_scrobble = false;
        }
    }

    async fn position_changed(&mut self, position: u64) {
        if position < self.last_postion + 2 && position > self.last_postion {
            self.accumulated_time += position - self.last_postion;
        }

        self.last_postion = position;

        if self.duration >= 30
            && (self.accumulated_time > self.duration / 2 || self.accumulated_time > 240)
            && !self.should_scrobble
            && self.metadata.is_some()
        {
            self.should_scrobble = true;
        }
    }

    async fn duration_changed(&mut self, duration: u64) {
        self.duration = duration;
    }
}

impl Drop for LastFM {
    fn drop(&mut self) {
        if self.should_scrobble {
            debug!("attempting scrobble before dropping LastFM, this will block");
            block_on(self.scrobble());
        }
    }
}
