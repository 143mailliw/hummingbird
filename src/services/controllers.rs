#[cfg(target_os = "macos")]
mod macos;

use std::{path::Path, sync::mpsc::Sender};

use async_trait::async_trait;

use crate::{
    media::metadata::Metadata,
    playback::{
        events::{PlaybackCommand, RepeatState},
        thread::PlaybackState,
    },
};

pub trait InitPlaybackController {
    async fn init(bridge: ControllerBridge) -> Box<dyn PlaybackController>;
}

#[async_trait]
pub trait PlaybackController {
    async fn position_changed(&mut self, new_position: u64);
    async fn duration_changed(&mut self, new_duration: u64);
    async fn volume_changed(&mut self, new_volume: f64);
    async fn metadata_changed(&mut self, metadata: &Metadata);
    async fn album_art_changed(&mut self, album_art: &Box<[u8]>);
    async fn repeat_state_changed(&mut self, repeat_state: RepeatState);
    async fn playback_state_changed(&mut self, playback_state: PlaybackState);
    async fn new_file(&mut self, path: &Path);
}

pub struct ControllerBridge {
    playback_thread: Sender<PlaybackCommand>,
}

impl ControllerBridge {
    pub fn new(playback_thread: Sender<PlaybackCommand>) -> Self {
        Self { playback_thread }
    }

    pub fn play(&self) {
        self.playback_thread
            .send(PlaybackCommand::Play)
            .expect("could not send tx (from ControllerBridge)");
    }

    pub fn pause(&self) {
        self.playback_thread
            .send(PlaybackCommand::Pause)
            .expect("could not send tx (from ControllerBridge)");
    }

    pub fn stop(&self) {
        self.playback_thread
            .send(PlaybackCommand::Stop)
            .expect("could not send tx (from ControllerBridge)");
    }

    pub fn next(&self) {
        self.playback_thread
            .send(PlaybackCommand::Next)
            .expect("could not send tx (from ControllerBridge)");
    }

    pub fn previous(&self) {
        self.playback_thread
            .send(PlaybackCommand::Previous)
            .expect("could not send tx (from ControllerBridge)");
    }

    pub fn jump(&self, index: usize) {
        self.playback_thread
            .send(PlaybackCommand::Jump(index))
            .expect("could not send tx (from ControllerBridge)");
    }

    pub fn seek(&self, position: f64) {
        self.playback_thread
            .send(PlaybackCommand::Seek(position))
            .expect("could not send tx (from ControllerBridge)");
    }

    pub fn set_volume(&self, volume: f64) {
        self.playback_thread
            .send(PlaybackCommand::SetVolume(volume))
            .expect("could not send tx (from ControllerBridge)");
    }

    pub fn toggle_shuffle(&self) {
        self.playback_thread
            .send(PlaybackCommand::ToggleShuffle)
            .expect("could not send tx (from ControllerBridge)");
    }

    pub fn set_repeat(&self, repeat: RepeatState) {
        self.playback_thread
            .send(PlaybackCommand::SetRepeat(repeat))
            .expect("could not send tx (from ControllerBridge)");
    }
}
