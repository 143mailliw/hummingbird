#![allow(dead_code)]

use std::{path::PathBuf, sync::Arc};

use async_channel::{Receiver, Sender};
use gpui::App;

use crate::{
    playback::events::RepeatState,
    ui::models::{CurrentTrack, ImageEvent, MMBSEvent, Models, PlaybackInfo},
};

use super::{
    events::{PlaybackCommand, PlaybackEvent},
    queue::QueueItemData,
    thread::PlaybackState,
};

/// The PlaybackInterface trait defines the method used to create the struct that will be used to
/// communicate between the playback thread and the main thread.
pub trait PlaybackInterface {
    fn new(commands_tx: Sender<PlaybackCommand>, events_rx: Receiver<PlaybackEvent>) -> Self;
}

/// The playback interface struct that will be used to communicate between the playback thread and
/// the main thread. This implementation takes advantage of the GPUI Global trait to allow any
/// function (so long as it is running on the main thread) to send commands to the playback thread.
///
/// This interface takes advantage of GPUI's asynchronous runtime to read messages without blocking
/// rendering. Messages are read at quickest every 10ms, however the runtime may choose to run the
/// function that reads events less frequently, depending on the current workload. Because of this,
/// event handling should not perform any heavy operations, which should be instead sent to the
/// data thread for any required additional processing.
///
/// For the functions provided by this interface, see the documentation for the playback thread.
pub struct GPUIPlaybackInterface {
    commands_tx: Sender<PlaybackCommand>,
    events_rx: Option<Receiver<PlaybackEvent>>,
}

impl gpui::Global for GPUIPlaybackInterface {}

impl PlaybackInterface for GPUIPlaybackInterface {
    fn new(commands_tx: Sender<PlaybackCommand>, events_rx: Receiver<PlaybackEvent>) -> Self {
        Self {
            commands_tx,
            events_rx: Some(events_rx),
        }
    }
}

impl GPUIPlaybackInterface {
    pub fn play(&self) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::Play)
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn pause(&self) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::Pause)
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn open(&self, path: PathBuf) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::Open(path))
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn queue(&self, item: QueueItemData) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::Queue(item))
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn queue_list(&self, items: Vec<QueueItemData>) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::QueueList(items))
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn next(&self) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::Next)
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn previous(&self) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::Previous)
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn clear_queue(&self) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::ClearQueue)
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn jump(&self, index: usize) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::Jump(index))
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn jump_unshuffled(&self, index: usize) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::JumpUnshuffled(index))
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn seek(&self, position: f64) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::Seek(position))
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn set_volume(&self, volume: f64) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::SetVolume(volume))
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn replace_queue(&self, items: Vec<QueueItemData>) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::ReplaceQueue(items))
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn stop(&self) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::Stop)
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn toggle_shuffle(&self) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::ToggleShuffle)
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn set_repeat(&self, state: RepeatState) {
        let commands_tx = self.commands_tx.clone();
        smol::spawn(async move {
            commands_tx
                .send(PlaybackCommand::SetRepeat(state))
                .await
                .expect("could not send tx");
        })
        .detach();
    }

    pub fn get_sender(&self) -> Sender<PlaybackCommand> {
        self.commands_tx.clone()
    }

    /// Starts the broadcast loop that will read events from the playback thread and update data
    /// models accordingly. This function should be called once, and will panic if called more than
    /// once.
    pub fn start_broadcast(&mut self, app: &mut App) {
        // This function's sole responsibility is to read events from the playback thread and update
        // data models accordingly.
        let mut events_rx = None;
        std::mem::swap(&mut self.events_rx, &mut events_rx);

        let metadata_model = app.global::<Models>().metadata.clone();
        let albumart_model = app.global::<Models>().albumart.clone();
        let queue_model = app.global::<Models>().queue.clone();
        let mmbs_model = app.global::<Models>().mmbs.clone();

        let playback_info = app.global::<PlaybackInfo>().clone();

        let Some(events_rx) = events_rx else {
            panic!("broadcast thread already started");
        };

        app.spawn(async move |cx| {
            loop {
                while let Ok(event) = events_rx.recv().await {
                    match event {
                        PlaybackEvent::MetadataUpdate(v) => {
                            let metadata = Arc::new(*v.clone());

                            metadata_model
                                .update(cx, |m, cx| {
                                    *m = *v;
                                    cx.notify()
                                })
                                .expect("failed to update metadata");

                            mmbs_model
                                .update(cx, |_, cx| {
                                    cx.emit(MMBSEvent::MetadataRecieved(metadata));
                                })
                                .expect("failed to broadcast MMBS event MetadataRecieved");
                        }
                        PlaybackEvent::AlbumArtUpdate(v) => {
                            albumart_model
                                .update(cx, |m, cx| {
                                    if let Some(v) = v {
                                        cx.emit(ImageEvent(v))
                                    } else {
                                        *m = None;
                                        cx.notify()
                                    }
                                })
                                .expect("failed to update albumart");
                        }
                        PlaybackEvent::StateChanged(v) => {
                            playback_info
                                .playback_state
                                .update(cx, |m, cx| {
                                    *m = v;
                                    cx.notify()
                                })
                                .expect("failed to update playback state");

                            if v == PlaybackState::Stopped {
                                playback_info
                                    .current_track
                                    .update(cx, |m, cx| {
                                        *m = None;
                                        cx.notify()
                                    })
                                    .expect("failed to update current track");
                            }

                            mmbs_model
                                .update(cx, |_, cx| {
                                    cx.emit(MMBSEvent::StateChanged(v));
                                })
                                .expect("failed to broadcast MMBS event StateChanged");
                        }
                        PlaybackEvent::PositionChanged(v) => {
                            playback_info
                                .position
                                .update(cx, |m, cx| {
                                    *m = v;
                                    cx.notify()
                                })
                                .expect("failed to update position");
                            mmbs_model
                                .update(cx, |_, cx| {
                                    cx.emit(MMBSEvent::PositionChanged(v));
                                })
                                .expect("failed to broadcast MMBS event PositionChanged");
                        }
                        PlaybackEvent::DurationChanged(v) => {
                            playback_info
                                .duration
                                .update(cx, |m, cx| {
                                    *m = v;
                                    cx.notify()
                                })
                                .expect("failed to update duration");
                            mmbs_model
                                .update(cx, |_, cx| {
                                    cx.emit(MMBSEvent::DurationChanged(v));
                                })
                                .expect("failed to broadcast MMBS event DurationChanged");
                        }
                        PlaybackEvent::SongChanged(path) => {
                            playback_info
                                .current_track
                                .update(cx, |m, cx| {
                                    *m = Some(CurrentTrack::new(path.clone()));
                                    cx.notify()
                                })
                                .expect("failed to update current track");
                            mmbs_model
                                .update(cx, |_, cx| {
                                    cx.emit(MMBSEvent::NewTrack(path));
                                })
                                .expect("failed to broadcast MMBS event NewTrack");
                        }
                        PlaybackEvent::QueueUpdated => {
                            queue_model
                                .update(cx, |_, cx| cx.notify())
                                .expect("failed to update queue");
                        }
                        PlaybackEvent::ShuffleToggled(v, _) => {
                            playback_info
                                .shuffling
                                .update(cx, |m, cx| {
                                    *m = v;
                                    cx.notify()
                                })
                                .expect("failed to update shuffle state");
                        }
                        PlaybackEvent::VolumeChanged(v) => {
                            playback_info
                                .volume
                                .update(cx, |m, cx| {
                                    *m = v;
                                    cx.notify()
                                })
                                .expect("failed to update volume model");

                            // Note: `prev_volume` should not be to small.
                            // Its value needs to be visible in UI
                            // while toggling volume `on` / `off` and even
                            // an user used a slider to move volume to `0`
                            if v > 0.05 {
                                playback_info
                                    .prev_volume
                                    .update(cx, |m, cx| {
                                        *m = v;
                                        cx.notify()
                                    })
                                    .expect("failed to update volume model");
                            }
                        }
                        PlaybackEvent::QueuePositionChanged(v) => queue_model
                            .update(cx, |m, cx| {
                                m.position = v;
                                cx.notify();
                            })
                            .expect("failed to update queue position"),
                        PlaybackEvent::RepeatChanged(v) => playback_info
                            .repeating
                            .update(cx, |m, cx| {
                                *m = v;
                                cx.notify();
                            })
                            .expect("failed to update repeat model"),
                    }
                }
            }
        })
        .detach();
    }
}

// TODO: this should be in a trait for AppContext
/// Replace the current queue with the given items.
pub fn replace_queue(items: Vec<QueueItemData>, app: &mut App) {
    let playback_interface = app.global::<GPUIPlaybackInterface>();
    playback_interface.replace_queue(items);

    // let data_interface = app.global::<GPUIDataInterface>();

    // data_interface.evict_cache();
}
