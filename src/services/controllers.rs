#[cfg(target_os = "macos")]
mod macos;

use std::{
    path::Path,
    sync::{mpsc::Sender, Arc},
};

use ahash::AHashMap;
use async_lock::Mutex;
use async_trait::async_trait;
use gpui::{App, AppContext, Entity, Global};

use crate::{
    media::metadata::Metadata,
    playback::{
        events::{PlaybackCommand, RepeatState},
        interface::GPUIPlaybackInterface,
        thread::PlaybackState,
    },
    ui::models::{Models, PlaybackInfo},
};

pub trait InitPlaybackController {
    fn init(bridge: ControllerBridge) -> Arc<Mutex<dyn PlaybackController>>;
}

#[async_trait]
pub trait PlaybackController {
    async fn position_changed(&mut self, new_position: u64);
    async fn duration_changed(&mut self, new_duration: u64);
    async fn volume_changed(&mut self, new_volume: f64);
    async fn metadata_changed(&mut self, metadata: &Metadata);
    async fn album_art_changed(&mut self, album_art: &[u8]);
    async fn repeat_state_changed(&mut self, repeat_state: RepeatState);
    async fn playback_state_changed(&mut self, playback_state: PlaybackState);
    async fn new_file(&mut self, path: &Path);
}

#[derive(Clone)]
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

pub type ControllerList = AHashMap<String, Arc<Mutex<dyn PlaybackController>>>;

pub struct CLHolder(pub Entity<ControllerList>);

impl Global for CLHolder {}

pub fn make_cl(cx: &mut App) {
    // cloning actually is neccesary because of the async move closure in pc_mutex
    #[allow(clippy::unnecessary_to_owned)]
    let cl = cx.new(|cx| {
        let models = cx.global::<Models>();
        let metadata = models.metadata.clone();
        let albumart = models.albumart.clone();

        cx.observe(&metadata, |m: &mut ControllerList, e, cx| {
            let metadata = Arc::new(e.read(cx).clone());

            for pc_mutex in m.values().cloned() {
                let metadata = metadata.clone();
                cx.spawn(async move |_, _| {
                    let mut pc = pc_mutex.lock().await;
                    pc.metadata_changed(&metadata).await;
                })
                .detach();
            }
        })
        .detach();

        cx.subscribe(&albumart, |m, _, ev, cx| {
            let art = Arc::new(ev.0.clone());

            for pc_mutex in m.values().cloned() {
                let art = art.clone();
                cx.spawn(async move |_, _| {
                    let mut pc = pc_mutex.lock().await;
                    pc.album_art_changed(&art).await;
                })
                .detach();
            }
        })
        .detach();

        let playback_info = cx.global::<PlaybackInfo>();
        let position = playback_info.position.clone();
        let duration = playback_info.duration.clone();
        let track = playback_info.current_track.clone();
        let volume = playback_info.volume.clone();
        let repeat = playback_info.repeating.clone();
        let state = playback_info.playback_state.clone();

        cx.observe(&position, |m: &mut ControllerList, e, cx| {
            let position = *e.read(cx);

            for pc_mutex in m.values().cloned() {
                cx.spawn(async move |_, _| {
                    let mut pc = pc_mutex.lock().await;
                    pc.position_changed(position).await;
                })
                .detach();
            }
        })
        .detach();

        cx.observe(&duration, |m: &mut ControllerList, e, cx| {
            let duration = *e.read(cx);

            for pc_mutex in m.values().cloned() {
                cx.spawn(async move |_, _| {
                    let mut pc = pc_mutex.lock().await;
                    pc.duration_changed(duration).await;
                })
                .detach();
            }
        })
        .detach();

        cx.observe(&track, |m: &mut ControllerList, e, cx| {
            if let Some(track) = e.read(cx) {
                let path = Arc::new(track.get_path().clone());

                for pc_mutex in m.values().cloned() {
                    let path = path.clone();
                    cx.spawn(async move |_, _| {
                        let mut pc = pc_mutex.lock().await;
                        pc.new_file(&path).await;
                    })
                    .detach();
                }
            }
        })
        .detach();

        cx.observe(&volume, |m: &mut ControllerList, e, cx| {
            let volume = *e.read(cx);

            for pc_mutex in m.values().cloned() {
                cx.spawn(async move |_, _| {
                    let mut pc = pc_mutex.lock().await;
                    pc.volume_changed(volume).await;
                })
                .detach();
            }
        })
        .detach();

        cx.observe(&repeat, |m: &mut ControllerList, e, cx| {
            let repeat = *e.read(cx);

            for pc_mutex in m.values().cloned() {
                cx.spawn(async move |_, _| {
                    let mut pc = pc_mutex.lock().await;
                    pc.repeat_state_changed(repeat).await;
                })
                .detach();
            }
        })
        .detach();

        cx.observe(&state, |m: &mut ControllerList, e, cx| {
            let state = *e.read(cx);

            for pc_mutex in m.values().cloned() {
                cx.spawn(async move |_, _| {
                    let mut pc = pc_mutex.lock().await;
                    pc.playback_state_changed(state).await;
                })
                .detach();
            }
        })
        .detach();

        let mut list = ControllerList::new();

        #[cfg(target_os = "macos")]
        {
            let sender = cx.global::<GPUIPlaybackInterface>().get_sender();
            let bridge = ControllerBridge {
                playback_thread: sender,
            };
            let macos_pc = macos::MacMediaPlayerController::init(bridge);

            list.insert("macos".to_string(), macos_pc);
        }

        list
    });

    cx.set_global(CLHolder(cl));
}
