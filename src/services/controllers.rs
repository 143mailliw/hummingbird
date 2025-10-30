#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod mpris;
#[cfg(target_os = "windows")]
mod windows;

use std::{path::Path, sync::Arc};

use ahash::AHashMap;
use async_channel::Sender;
use async_lock::Mutex;
use async_trait::async_trait;
use gpui::{App, AppContext, Entity, Global, Window};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tracing::{error, warn};

use crate::{
    media::metadata::Metadata,
    playback::{
        events::{PlaybackCommand, RepeatState},
        interface::GPUIPlaybackInterface,
        thread::PlaybackState,
    },
    ui::models::{Models, PlaybackInfo},
};

/// The InitPlaybackController trait allows you to initialize a new PlaybackController. All
/// PlaybackControllers must implement this trait.
///
/// A ControllerBridge is provided to allow external controllers to send playback events to the
/// playback thread, and a RawWindowHandle is provided to allow the controller to attach to the
/// window if necessary.
pub trait InitPlaybackController {
    /// Create a new PlaybackController.
    fn init(
        bridge: ControllerBridge,
        handle: Option<RawWindowHandle>,
    ) -> anyhow::Result<Arc<Mutex<dyn PlaybackController>>>;
}

#[async_trait]
/// The PlaybackController trait allows you to connect external controllers (like the system's
/// media controls) to Hummingbird.
///
/// When a new file is opened, events are emitted in this order:
/// new_file -> duration_changed -> metadata_changed -> album_art_changed, with metadata_changed
/// and album_art_changed occuring only if the track being played has metadata and album art,
/// respectively. Not all tracks will have metadata: you should still display the file name for
/// a track and allow controlling of playback.
///
/// PlaybackControllers are created via the InitPlaybackController trait, which is seperate to
/// allow PlaybackController to be object-safe.
///
/// Multiple PlaybackControllers can be attached at once; they will all be sent the same events and
/// the same data. Not all PlaybackControllers must handle all events - if you wish not to handle
/// a given event, simply implement the function by returning Ok(()).
///
/// All implementations of this trait should be proceeded by `#[async_trait]`, from the async-trait
/// library.
pub trait PlaybackController {
    /// Indicates that the position in the current file has changed.
    async fn position_changed(&mut self, new_position: u64) -> anyhow::Result<()>;

    /// Indicates that the duration of the current file has changed. This should only occur once
    /// per file.
    async fn duration_changed(&mut self, new_duration: u64) -> anyhow::Result<()>;

    /// Indicates that the playback volume has changed.
    async fn volume_changed(&mut self, new_volume: f64) -> anyhow::Result<()>;

    /// Indicates that new metadata has been recieved from the decoder. This may occur more than
    /// once per track.
    async fn metadata_changed(&mut self, metadata: &Metadata) -> anyhow::Result<()>;

    /// Indicates that new album art has been recieved from the decoder. This may occur more than
    /// once per track.
    async fn album_art_changed(&mut self, album_art: &[u8]) -> anyhow::Result<()>;

    /// Indicates that the repeat state has changed.
    async fn repeat_state_changed(&mut self, repeat_state: RepeatState) -> anyhow::Result<()>;

    /// Indicates that the playback state has changed. When the PlaybackState is Stopped, no file
    /// is queued for playback.
    async fn playback_state_changed(&mut self, playback_state: PlaybackState)
    -> anyhow::Result<()>;

    /// Indicates that the shuffle state has changed.
    async fn shuffle_state_changed(&mut self, shuffling: bool) -> anyhow::Result<()>;

    /// Indicates that a new file has started playing. The metadata, duration, position, and album
    /// art should be reset to default/empty values when this event is recieved.
    async fn new_file(&mut self, path: &Path) -> anyhow::Result<()>;
}

#[derive(Clone)]
pub struct ControllerBridge {
    playback_thread: Sender<PlaybackCommand>,
}

#[allow(dead_code)]
impl ControllerBridge {
    pub fn new(playback_thread: Sender<PlaybackCommand>) -> Self {
        Self { playback_thread }
    }

    pub fn play(&self) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::Play)
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }

    pub fn pause(&self) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::Pause)
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }

    pub fn toggle_play_pause(&self) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::TogglePlayPause)
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }

    pub fn stop(&self) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::Stop)
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }

    pub fn next(&self) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::Next)
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }

    pub fn previous(&self) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::Previous)
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }

    pub fn jump(&self, index: usize) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::Jump(index))
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }

    pub fn seek(&self, position: f64) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::Seek(position))
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }

    pub fn set_volume(&self, volume: f64) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::SetVolume(volume))
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }

    pub fn toggle_shuffle(&self) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::ToggleShuffle)
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }

    pub fn set_repeat(&self, repeat: RepeatState) {
        let playback_thread = self.playback_thread.clone();
        smol::spawn(async move {
            playback_thread
                .send(PlaybackCommand::SetRepeat(repeat))
                .await
                .expect("could not send tx (from ControllerBridge)");
        })
        .detach();
    }
}

pub type ControllerList = AHashMap<String, Arc<Mutex<dyn PlaybackController>>>;

// has to be held in memory
#[allow(dead_code)]
pub struct CLHolder(pub Entity<ControllerList>);

impl Global for CLHolder {}

pub fn make_cl(cx: &mut App, window: &mut Window) {
    let rwh = if cfg!(target_os = "linux") {
        // X11 windows panic with unimplemented and we don't need it here
        None
    } else {
        HasWindowHandle::window_handle(window)
            .ok()
            .map(|v| v.as_raw())
    };

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
                    if let Err(err) = pc.metadata_changed(&metadata).await {
                        error!("Error updating metadata for PC: {}", err);
                    };
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
                    if let Err(err) = pc.album_art_changed(&art).await {
                        error!("Error updating album art for PC: {}", err);
                    };
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
        let shuffle = playback_info.shuffling.clone();

        cx.observe(&position, |m: &mut ControllerList, e, cx| {
            let position = *e.read(cx);

            for pc_mutex in m.values().cloned() {
                cx.spawn(async move |_, _| {
                    let mut pc = pc_mutex.lock().await;
                    if let Err(err) = pc.position_changed(position).await {
                        error!("Error updating position for PC: {}", err);
                    };
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
                    if let Err(err) = pc.duration_changed(duration).await {
                        error!("Error updating duration for PC: {}", err);
                    };
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
                        if let Err(err) = pc.new_file(&path).await {
                            error!("Error submitting new file to PC: {}", err);
                        };
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
                    if let Err(err) = pc.volume_changed(volume).await {
                        error!("Error updating volume for PC: {}", err);
                    };
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
                    if let Err(err) = pc.repeat_state_changed(repeat).await {
                        error!("Error updating repeat state for PC: {}", err);
                    };
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
                    if let Err(err) = pc.playback_state_changed(state).await {
                        error!("Error updating playback state for PC: {}", err);
                    };
                })
                .detach();
            }
        })
        .detach();

        cx.observe(&shuffle, |m: &mut ControllerList, e, cx| {
            let shuffle = *e.read(cx);

            for pc_mutex in m.values().cloned() {
                cx.spawn(async move |_, _| {
                    let mut pc = pc_mutex.lock().await;
                    if let Err(err) = pc.shuffle_state_changed(shuffle).await {
                        error!("Error updating shuffle state for PC: {}", err)
                    }
                })
                .detach();
            }
        })
        .detach();

        let mut list = ControllerList::new();

        let sender = cx.global::<GPUIPlaybackInterface>().get_sender();
        let bridge = ControllerBridge::new(sender);

        #[cfg(target_os = "macos")]
        {
            if let Ok(macos_pc) = macos::MacMediaPlayerController::init(bridge, rwh) {
                list.insert("macos".to_string(), macos_pc);
            } else {
                error!("Failed to initialize MacMediaPlayerController!");
                warn!("Desktop integration will be unavailable.");
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(mpris_pc) = mpris::MprisController::init(bridge, rwh) {
                list.insert("mpris".to_string(), mpris_pc);
            } else {
                error!("Failed to initialize MprisController!");
                warn!("Desktop integration will be unavailable.");
            };
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(windows_pc) = windows::WindowsController::init(bridge, rwh) {
                list.insert("windows".to_string(), windows_pc);
            } else {
                error!("Failed to initialize WindowsController!");
                warn!("Desktop integration will be unavailable.");
            };
        }

        list
    });

    cx.set_global(CLHolder(cl));
}
