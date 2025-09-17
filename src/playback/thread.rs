use std::{
    env::consts::OS,
    mem::swap,
    path::PathBuf,
    sync::{Arc, RwLock},
    thread::sleep,
};

use async_channel::{Receiver, Sender};
use rand::{rng, seq::SliceRandom};
use tracing::{debug, error, info, warn};

use crate::{devices::builtin::cpal::CpalProvider, playback::events::RepeatState};
use crate::{devices::builtin::dummy::DummyDeviceProvider, settings::playback::PlaybackSettings};
// #[cfg(target_os = "linux")]
// use crate::devices::builtin::pulse::PulseProvider;
#[cfg(target_os = "windows")]
use crate::devices::builtin::win_audiograph::AudioGraphProvider;

use crate::{
    devices::{
        format::{ChannelSpec, FormatInfo},
        resample::Resampler,
        traits::{Device, DeviceProvider, OutputStream},
    },
    media::{
        builtin::symphonia::SymphoniaProvider, errors::PlaybackReadError, traits::MediaProvider,
    },
};

use super::{
    events::{PlaybackCommand, PlaybackEvent},
    interface::PlaybackInterface,
    queue::QueueItemData,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

pub struct PlaybackThread {
    /// The playback settings. Recieved on thread startup.
    playback_settings: PlaybackSettings,

    /// The command receiver.
    commands_rx: Receiver<PlaybackCommand>,

    /// The event sender.
    events_tx: Sender<PlaybackEvent>,

    /// The current media provider.
    ///
    /// In the future this will be a hash map of media providers,
    /// allowing for multiple media providers to be used simultaneously.
    media_provider: Option<Box<dyn MediaProvider>>,

    /// The current device provider.
    device_provider: Option<Box<dyn DeviceProvider>>,

    /// The current device.
    device: Option<Box<dyn Device>>,

    /// The current stream.
    ///
    /// Note: This stream may become invalid (depending on the device provider). It is the
    /// responsibility of the playback thread to handle this, so you should handle errors
    /// gracefully.
    stream: Option<Box<dyn OutputStream>>,

    /// The current playback state (playing, paused, stopped).
    state: PlaybackState,

    /// The current resampler, if one exists. This is used to convert the audio format of the media
    /// to the format supported by the device. Note that the resampler should always be called
    /// before writing to the device, even if the device uses the same format as the media, as the
    /// resampler will not perform any operations if the formats are the same.
    resampler: Option<Resampler>,

    /// The current format of the media.
    format: Option<FormatInfo>,

    /// The current queue. Do not hold an indefinite lock on this queue - it is read by the
    /// UI thread.
    queue: Arc<RwLock<Vec<QueueItemData>>>,

    /// If the queue is shuffled, this is a copy of the original (unshuffled) queue.
    original_queue: Vec<QueueItemData>,

    /// Whether or not the queue is shuffled.
    shuffle: bool,

    /// The index after the current item in the queue. This can be out of bounds if the current
    /// track is the last track in the queue.
    queue_next: usize,

    /// The last timestamp of the current track. This is used to determine if the position has
    /// changed since the last update.
    last_timestamp: u64,

    /// Whether or not the stream should be reset before playback is continued.
    pending_reset: bool,

    /// Whether or not the queue should be repeated when the end of the queue is reached.
    repeat: RepeatState,
}

pub const LN_50: f64 = 3.91202300543_f64;
pub const LINEAR_SCALING_COEFFICIENT: f64 = 0.295751527165_f64;

impl PlaybackThread {
    /// Starts the playback thread and returns the created interface.
    pub fn start<T: PlaybackInterface>(
        queue: Arc<RwLock<Vec<QueueItemData>>>,
        settings: PlaybackSettings,
    ) -> T {
        // TODO: use the refresh rate for the bounds
        let (commands_tx, commands_rx) = async_channel::unbounded();
        let (events_tx, events_rx) = async_channel::unbounded();

        std::thread::Builder::new()
            .name("playback".to_string())
            .spawn(move || {
                let mut thread = PlaybackThread {
                    commands_rx,
                    events_tx,
                    media_provider: None,
                    device_provider: None,
                    device: None,
                    stream: None,
                    state: PlaybackState::Stopped,
                    resampler: None,
                    format: None,
                    queue,
                    original_queue: Vec::new(),
                    shuffle: false,
                    queue_next: 0,
                    last_timestamp: u64::MAX,
                    pending_reset: false,
                    repeat: if settings.always_repeat {
                        RepeatState::Repeating
                    } else {
                        RepeatState::NotRepeating
                    },
                    playback_settings: settings,
                };

                thread.run();
            })
            .expect("could not start playback thread");

        T::new(commands_tx, events_rx)
    }

    /// Creates the initial stream and starts the main loop.
    pub fn run(&mut self) {
        // for now just throw in the default Providers and pick the default Device
        // TODO: Add a way to select the output device
        // #[cfg(target_os = "linux")]
        // {
        //     self.device_provider = Some(Box::new(PulseProvider::default()));
        // }
        // #[cfg(target_os = "windows")]
        // {
        //     if option_env!("USE_CPAL_WASAPI").is_some() {
        //         self.device_provider = Some(Box::new(CpalProvider::default()));
        //     } else {
        //         self.device_provider = Some(Box::new(AudioGraphProvider::default()));
        //     }
        // }
        // #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
        // {
        //     self.device_provider = Some(Box::new(CpalProvider::default()));
        // }

        let default_device_provider = match OS {
            "linux" => "cpal", // TODO: reimplement pulse provider
            "windows" => "win_audiograph",
            _ => "cpal",
        };

        let requested_device_provider = std::env::var("DEVICE_PROVIDER")
            .unwrap_or_else(|_| default_device_provider.to_string());

        match requested_device_provider.as_str() {
            "pulse" => {
                // #[cfg(target_os = "linux")]
                // {
                //     self.device_provider = Some(Box::new(PulseProvider::default()));
                // }
                // #[cfg(not(target_os = "linux"))]
                // {
                //     warn!("pulse is not supported on this platform");
                //     warn!("Falling back to CPAL");
                //     self.device_provider = Some(Box::new(CpalProvider::default()));
                // }
                warn!("pulseaudio support was removed");
                warn!("Falling back to CPAL");
                self.device_provider = Some(Box::new(CpalProvider::default()));
            }
            "win_audiograph" => {
                #[cfg(target_os = "windows")]
                {
                    self.device_provider = Some(Box::new(AudioGraphProvider::default()));
                }
                #[cfg(not(target_os = "windows"))]
                {
                    warn!("win_audiograph is not supported on this platform");
                    warn!("Falling back to CPAL");
                    self.device_provider = Some(Box::new(CpalProvider::default()));
                }
            }
            "cpal" => {
                self.device_provider = Some(Box::new(CpalProvider::default()));
            }
            "dummy" => {
                self.device_provider = Some(Box::new(DummyDeviceProvider::new()));
            }
            _ => {
                warn!("Unknown device provider: {}", requested_device_provider);
                warn!("Falling back to CPAL");
                self.device_provider = Some(Box::new(CpalProvider::default()));
            }
        }

        self.media_provider = Some(Box::new(SymphoniaProvider::default()));

        // TODO: allow the user to pick a format on supported platforms
        self.recreate_stream(true, None);

        loop {
            self.main_loop();
        }
    }

    /// Start command intake and audio playback loop.
    pub fn main_loop(&mut self) {
        self.command_intake();

        if self.state == PlaybackState::Playing {
            self.play_audio();
        } else {
            sleep(std::time::Duration::from_millis(10));
        }

        self.broadcast_events();
    }

    /// Check for updated metadata and album art, and broadcast it to the UI.
    pub fn broadcast_events(&mut self) {
        let Some(provider) = &mut self.media_provider else {
            return;
        };
        if !provider.metadata_updated() {
            return;
        }
        // TODO: proper error handling
        let metadata = Box::new(
            provider
                .read_metadata()
                .expect("failed to get metadata")
                .clone(),
        );
        let events_tx = self.events_tx.clone();
        smol::spawn(async move {
            events_tx
                .send(PlaybackEvent::MetadataUpdate(metadata))
                .await
                .expect("unable to send event");
        })
        .detach();

        let image = provider.read_image().expect("failed to decode image");
        let events_tx = self.events_tx.clone();
        smol::spawn(async move {
            events_tx
                .send(PlaybackEvent::AlbumArtUpdate(image))
                .await
                .expect("unable to send event");
        })
        .detach();
    }

    /// Read incoming commands from the command channel, and process them.
    pub fn command_intake(&mut self) {
        while let Ok(command) = self.commands_rx.try_recv() {
            match command {
                PlaybackCommand::Play => self.play(),
                PlaybackCommand::Pause => self.pause(),
                PlaybackCommand::TogglePlayPause => self.toggle_play_pause(),
                PlaybackCommand::Open(path) => self.open(&path),
                PlaybackCommand::Queue(v) => self.queue(v),
                PlaybackCommand::QueueList(v) => self.queue_list(v),
                PlaybackCommand::Next => self.next(true),
                PlaybackCommand::Previous => self.previous(),
                PlaybackCommand::ClearQueue => self.clear_queue(),
                PlaybackCommand::Jump(v) => self.jump(v),
                PlaybackCommand::JumpUnshuffled(v) => self.jump_unshuffled(v),
                PlaybackCommand::Seek(v) => self.seek(v),
                PlaybackCommand::SetVolume(v) => self.set_volume(v),
                PlaybackCommand::ReplaceQueue(v) => self.replace_queue(v),
                PlaybackCommand::Stop => self.stop(),
                PlaybackCommand::ToggleShuffle => self.toggle_shuffle(),
                PlaybackCommand::SetRepeat(v) => self.set_repeat(v),
            }
        }
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Paused {
            return;
        }

        if self.state == PlaybackState::Playing {
            if let Some(stream) = &mut self.stream {
                // stream is being played right now which means it has to be valid
                // this is fine
                stream.pause().expect("unable to pause stream");
            }

            self.state = PlaybackState::Paused;

            let events_tx = self.events_tx.clone();
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::StateChanged(PlaybackState::Paused))
                    .await
                    .expect("unable to send event");
            })
            .detach();
        }
    }

    /// Resume playback. If the last track was the end of the queue, the queue will be restarted.
    pub fn play(&mut self) {
        if self.state == PlaybackState::Playing {
            return;
        }

        if self.state == PlaybackState::Paused {
            if self.stream.is_some() {
                if self.pending_reset {
                    // we have to do .as_mut.unwrap() because we need self later
                    let result = self.stream.as_mut().unwrap().reset();

                    if let Err(err) = result {
                        let format = self.format.clone();
                        warn!(
                            "Failed to reset stream, recreating device instead... {:?}",
                            err
                        );
                        self.recreate_stream(true, format.map(|v| v.channels));
                    }

                    self.pending_reset = false;
                }

                let result = self.stream.as_mut().unwrap().play();
                if let Err(err) = result {
                    let format = self.format.clone();
                    warn!(
                        "Failed to restart playback, recreating device and retrying... {:?}",
                        err
                    );
                    self.recreate_stream(true, format.map(|v| v.channels));
                    let final_result = self.stream.as_mut().unwrap().play();

                    if final_result.is_err() {
                        error!("Failed to start playback after recreation");
                        error!("This likely indicates a problem with the audio device or driver");
                        error!("(or an underlying issue in the used DeviceProvider)");
                        error!("Please check your audio setup and try again.");
                        panic!("Failed to submit frame after recreation");
                    }
                }
            }

            self.state = PlaybackState::Playing;

            let events_tx = self.events_tx.clone();
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::StateChanged(PlaybackState::Playing))
                    .await
                    .expect("unable to send event");
            })
            .detach();
        }

        let queue = self.queue.read().expect("couldn't get the queue");

        if self.state == PlaybackState::Stopped && !queue.is_empty() {
            let path = queue[0].get_path().clone();
            drop(queue);
            self.open(&path);
            let events_tx = self.events_tx.clone();
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::QueuePositionChanged(0))
                    .await
                    .expect("unable to send event");
            })
            .detach();
            self.queue_next = 1;
        }

        // nothing to play, womp womp
    }

    /// Open a new track by given path.
    fn open(&mut self, path: &PathBuf) {
        info!("Opening: {:?}", path);

        let mut recreation_required = false;

        if self.state == PlaybackState::Paused {
            let result = self.stream.as_mut().unwrap().reset();

            if let Err(err) = result {
                warn!("Failed to reset device, forcing recreation: {:?}", err);
                recreation_required = true;
            }
        }

        let play_result = self.stream.as_mut().unwrap().play();

        if play_result.is_err() {
            warn!(
                "Failed to start playback, forcing recreation: {:?}",
                play_result.err().unwrap()
            );
            recreation_required = true;
        }

        // TODO: handle multiple media providers
        let Some(provider) = &mut self.media_provider else {
            return;
        };
        // TODO: proper error handling
        self.resampler = None;
        let src = std::fs::File::open(path).expect("failed to open media");
        provider.open(src, None).expect("unable to open file");
        provider.start_playback().expect("unable to start playback");

        let channels = provider.channels().expect("unable to get channels");
        let stream_channels = self
            .stream
            .as_ref()
            .unwrap()
            .get_current_format()
            .unwrap()
            .channels
            .clone();

        if channels.count() != stream_channels.count() {
            info!(
                "Channel count mismatch, re-opening with the correct channel count (if supported)"
            );
            info!(
                "Decoder wanted {}, stream had {}",
                channels.count(),
                stream_channels.count()
            );
            recreation_required = true;
        }

        let events_tx = self.events_tx.clone();
        let path = path.clone();
        smol::spawn(async move {
            events_tx
                .send(PlaybackEvent::SongChanged(path))
                .await
                .expect("unable to send event");
        })
        .detach();

        if let Ok(duration) = provider.duration_secs() {
            let events_tx = self.events_tx.clone();
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::DurationChanged(duration))
                    .await
                    .expect("unable to send event");
            })
            .detach();
        } else {
            let events_tx = self.events_tx.clone();
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::DurationChanged(0))
                    .await
                    .expect("unable to send event");
            })
            .detach();
        }

        if recreation_required {
            self.recreate_stream(true, Some(channels));
            let play_result = self.stream.as_mut().unwrap().play();

            if play_result.is_err() {
                error!("Device was recreated and we still can't play");
                panic!("couldn't play device")
            }
        }

        self.state = PlaybackState::Playing;

        self.update_ts();

        let events_tx = self.events_tx.clone();
        smol::spawn(async move {
            events_tx
                .send(PlaybackEvent::StateChanged(PlaybackState::Playing))
                .await
                .expect("unable to send event");
        })
        .detach();
    }

    /// Skip to the next track in the queue.
    fn next(&mut self, user_initiated: bool) {
        let mut queue = self.queue.write().expect("couldn't get the queue");

        if self.repeat == RepeatState::RepeatingOne {
            info!("Repeating current track");
            let path = queue[self.queue_next - 1].get_path().clone();
            drop(queue);
            self.open(&path);
            return;
        }

        if self.queue_next < queue.len() {
            info!("Opening next file in queue");
            let path = queue[self.queue_next].get_path().clone();
            drop(queue);
            self.open(&path);
            let events_tx = self.events_tx.clone();
            let queue_next = self.queue_next;
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::QueuePositionChanged(queue_next))
                    .await
                    .expect("unable to send event");
            })
            .detach();
            self.queue_next += 1;
        } else if !user_initiated {
            if self.repeat == RepeatState::Repeating {
                info!("End of queue reached, repeating.");

                if self.shuffle {
                    queue.shuffle(&mut rng());

                    let events_tx = self.events_tx.clone();
                    smol::spawn(async move {
                        events_tx
                            .send(PlaybackEvent::QueueUpdated)
                            .await
                            .expect("unable to send event");
                    })
                    .detach();
                }

                drop(queue);
                self.jump(0);
            } else {
                info!("Playback queue is empty, stopping playback");
                drop(queue);
                self.stop();
            }
        }
    }

    /// Skip to the previous track in the queue.
    fn previous(&mut self) {
        if self.state == PlaybackState::Playing
            && self.playback_settings.prev_track_jump_first
            && self.last_timestamp > 5
        {
            self.seek(0_f64);
            return;
        }

        let queue = self.queue.read().expect("couldn't get the queue");

        if self.state == PlaybackState::Stopped && !queue.is_empty() {
            let path = queue.last().unwrap().get_path().clone();
            self.queue_next = queue.len();
            drop(queue);
            self.open(&path);
            let events_tx = self.events_tx.clone();
            let new_position = self.queue_next - 1;
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::QueuePositionChanged(new_position))
                    .await
                    .expect("unable to send event");
            })
            .detach();
        } else if self.queue_next > 1 {
            info!("Opening previous file in queue");
            let path = queue[self.queue_next - 2].get_path().clone();
            drop(queue);
            let events_tx = self.events_tx.clone();
            let new_position = self.queue_next - 2;
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::QueuePositionChanged(new_position))
                    .await
                    .expect("unable to send event");
            })
            .detach();
            self.queue_next -= 1;
            debug!("queue_next: {}", self.queue_next);
            self.open(&path);
        }
    }

    /// Add a new QueueItemData to the queue. If nothing is playing, start playing it.
    fn queue(&mut self, item: QueueItemData) {
        info!("Adding file to queue: {}", item);

        let mut queue = self.queue.write().expect("couldn't get the queue");

        let pre_len = queue.len();
        queue.push(item.clone());

        drop(queue);

        if self.shuffle {
            self.original_queue.push(item.clone());
        }

        if self.state == PlaybackState::Stopped {
            let path = item.get_path();
            self.open(path);
            self.queue_next = pre_len + 1;
            let events_tx = self.events_tx.clone();
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::QueuePositionChanged(pre_len))
                    .await
                    .expect("unable to send event");
            })
            .detach();
        }

        let events_tx = self.events_tx.clone();
        smol::spawn(async move {
            events_tx
                .send(PlaybackEvent::QueueUpdated)
                .await
                .expect("unable to send event");
        })
        .detach();
    }

    /// Add a list of QueueItemData to the queue. If nothing is playing, start playing the first
    /// track.
    fn queue_list(&mut self, mut paths: Vec<QueueItemData>) {
        info!("Adding files to queue: {:?}", paths);

        let mut queue = self.queue.write().expect("couldn't get the queue");

        let pre_len = queue.len();
        let first = paths.first().cloned();

        if self.shuffle {
            let mut shuffled_paths = paths.clone();
            shuffled_paths.shuffle(&mut rng());

            queue.append(&mut shuffled_paths);
            drop(queue);

            self.original_queue.append(&mut paths);
        } else {
            queue.append(&mut paths);
            drop(queue);
        }

        if self.state == PlaybackState::Stopped {
            if let Some(first) = first {
                let path = first.get_path();
                self.open(path);
                self.queue_next = pre_len + 1;
                let events_tx = self.events_tx.clone();
                smol::spawn(async move {
                    events_tx
                        .send(PlaybackEvent::QueuePositionChanged(pre_len))
                        .await
                        .expect("unable to send event");
                })
                .detach();
            }
        }

        let events_tx = self.events_tx.clone();
        smol::spawn(async move {
            events_tx
                .send(PlaybackEvent::QueueUpdated)
                .await
                .expect("unable to send event");
        })
        .detach();
    }

    /// Emit a PositionChanged event if the timestamp has changed.
    fn update_ts(&mut self) {
        if let Some(provider) = &self.media_provider {
            if let Ok(timestamp) = provider.position_secs() {
                if timestamp == self.last_timestamp {
                    return;
                }

                let events_tx = self.events_tx.clone();
                smol::spawn(async move {
                    events_tx
                        .send(PlaybackEvent::PositionChanged(timestamp))
                        .await
                        .expect("unable to send event");
                })
                .detach();

                self.last_timestamp = timestamp;
            }
        }
    }

    /// Seek to the specified timestamp (in seconds).
    fn seek(&mut self, timestamp: f64) {
        if let Some(provider) = &mut self.media_provider {
            provider.seek(timestamp).expect("unable to seek");
            self.pending_reset = true;
            self.update_ts();
        }
    }

    /// Jump to the specified index in the queue.
    fn jump(&mut self, index: usize) {
        let queue = self.queue.read().expect("couldn't get the queue");

        if index < queue.len() {
            let path = queue[index].get_path().clone();
            drop(queue);
            self.open(&path);
            self.queue_next = index + 1;
            let events_tx = self.events_tx.clone();
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::QueuePositionChanged(index))
                    .await
                    .expect("unable to send event");
            })
            .detach();
        }
    }

    /// Jump to the specified index in the queue, disregarding shuffling. This means that the
    /// original queue item at the specified index will be played, rather than the shuffled item.
    fn jump_unshuffled(&mut self, index: usize) {
        if !self.shuffle {
            self.jump(index);
            return;
        }

        let queue = self.queue.read().expect("couldn't get the queue");
        let path = self.original_queue[index].get_path();
        let pos = queue.iter().position(|a| a.get_path() == path);
        drop(queue);

        if let Some(pos) = pos {
            self.jump(pos);
        }
    }

    /// Replace the current queue with the given paths.
    fn replace_queue(&mut self, paths: Vec<QueueItemData>) {
        info!("Replacing queue with: {:?}", paths);

        let mut queue = self.queue.write().expect("couldn't get the queue");

        if self.shuffle {
            let mut shuffled_paths = paths.clone();
            shuffled_paths.shuffle(&mut rng());

            *queue = shuffled_paths;

            drop(queue);
            self.original_queue = paths;
        } else {
            *queue = paths;
            drop(queue);
        }

        self.queue_next = 0;
        self.jump(0);

        let events_tx = self.events_tx.clone();
        smol::spawn(async move {
            events_tx
                .send(PlaybackEvent::QueueUpdated)
                .await
                .expect("unable to send event");
        })
        .detach();
    }

    /// Clear the current queue.
    fn clear_queue(&mut self) {
        let mut queue = self.queue.write().expect("couldn't get the queue");
        *queue = Vec::new();
        self.original_queue = Vec::new();
        self.queue_next = 0;

        let events_tx = self.events_tx.clone();
        smol::spawn(async move {
            events_tx
                .send(PlaybackEvent::QueuePositionChanged(0))
                .await
                .expect("unable to send event");
            events_tx
                .send(PlaybackEvent::QueueUpdated)
                .await
                .expect("unable to send event");
        })
        .detach();
    }

    /// Stop the current playback.
    fn stop(&mut self) {
        if let Some(provider) = &mut self.media_provider {
            provider.stop_playback().expect("unable to stop playback");
            provider.close().expect("unable to close media");
        }
        self.state = PlaybackState::Stopped;

        let events_tx = self.events_tx.clone();
        smol::spawn(async move {
            events_tx
                .send(PlaybackEvent::StateChanged(PlaybackState::Stopped))
                .await
                .expect("unable to send event");
        })
        .detach();
    }

    /// Toggle shuffle mode. This will result in the queue being duplicated and shuffled.
    fn toggle_shuffle(&mut self) {
        let mut queue = self.queue.write().expect("couldn't get the queue");

        if self.shuffle {
            // find the current track in the unshuffled queue
            let index = if self.queue_next > 0 {
                let path = queue[self.queue_next - 1].get_path();
                let index = self
                    .original_queue
                    .iter()
                    .position(|x| x.get_path() == path)
                    .unwrap();
                self.queue_next = index + 1;
                index
            } else {
                0
            };

            swap(&mut self.original_queue, &mut queue);
            self.original_queue = Vec::new();
            self.shuffle = false;

            let events_tx = self.events_tx.clone();
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::ShuffleToggled(false, index))
                    .await
                    .expect("unable to send event");
                events_tx
                    .send(PlaybackEvent::QueueUpdated)
                    .await
                    .expect("unable to send event");
                if index != 0 {
                    events_tx
                        .send(PlaybackEvent::QueuePositionChanged(index))
                        .await
                        .expect("unable to send event");
                }
            })
            .detach();
        } else {
            self.original_queue = queue.clone();
            let length = queue.len();
            queue[self.queue_next..length].shuffle(&mut rng());
            self.shuffle = true;

            let events_tx = self.events_tx.clone();
            let queue_next = self.queue_next;
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::ShuffleToggled(true, queue_next))
                    .await
                    .expect("unable to send event");
                events_tx
                    .send(PlaybackEvent::QueueUpdated)
                    .await
                    .expect("unable to send event");
            })
            .detach();
        }
    }

    /// Sets the volume of the playback stream.
    fn set_volume(&mut self, volume: f64) {
        if let Some(stream) = self.stream.as_mut() {
            let volume_scaled = if volume >= 0.99_f64 {
                1_f64
            } else if volume > 0.1 {
                f64::exp(LN_50 * volume) / 50_f64
            } else {
                volume * LINEAR_SCALING_COEFFICIENT
            };

            stream
                .set_volume(volume_scaled)
                .expect("failed to set volume");

            let events_tx = self.events_tx.clone();
            smol::spawn(async move {
                events_tx
                    .send(PlaybackEvent::VolumeChanged(volume))
                    .await
                    .expect("unable to send event");
            })
            .detach();
        }
    }

    /// Sets the repeat mode. The queue will loop infinitely when repeat mode is enabled. When repeat once mode is enabled If shuffle
    /// mode is also enabled, the queue will be reshuffled when looped.
    fn set_repeat(&mut self, state: RepeatState) {
        self.repeat = if state == RepeatState::NotRepeating && self.playback_settings.always_repeat
        {
            RepeatState::Repeating
        } else {
            state
        };

        let events_tx = self.events_tx.clone();
        smol::spawn(async move {
            events_tx
                .send(PlaybackEvent::RepeatChanged(state))
                .await
                .expect("unable to send event");
        })
        .detach();
    }

    /// Toggles between play/pause.
    fn toggle_play_pause(&mut self) {
        match self.state {
            PlaybackState::Playing => self.pause(),
            PlaybackState::Paused => self.play(),
            _ => {}
        }
    }

    /// Recreates the playback stream with the given channels if any are provided, otherwise uses
    /// the device's default channel layout.
    fn recreate_stream(&mut self, force: bool, channels: Option<ChannelSpec>) {
        if let Some(mut stream) = self.stream.take() {
            stream.close_stream().expect("failed to close stream");
        }

        let Some(device_provider) = self.device_provider.as_mut() else {
            panic!("playback thread incorrectly initialized")
        };

        let Ok(mut device) = device_provider.get_default_device() else {
            error!("No playback device found, audio will not play");
            return;
        };

        if self.device.as_ref().and_then(|v| v.get_uid().ok()) == device.get_uid().ok() && !force {
            return;
        }

        let stream = if let Some(channels) = channels {
            let mut format = device
                .get_default_format()
                .expect("failed to get device format");

            if !format.rate_channel_ratio_fixed {
                let old_channels = format.channels.count();
                format.sample_rate =
                    (format.sample_rate / old_channels as u32) * channels.count() as u32;
                format.rate_channel_ratio = channels.count();
            }

            format.channels = channels;

            let result = device.open_device(format.clone());
            match result {
                Ok(stream) => stream,
                Err(err) => {
                    warn!(
                        "Failed to open device with requested format {:?}, error: {:?}",
                        format, err
                    );
                    warn!("Falling back to default format");
                    let format = device
                        .get_default_format()
                        .expect("failed to get device format");
                    device
                        .open_device(format)
                        .expect("failed to open device with default format")
                }
            }
        } else {
            let format = device
                .get_default_format()
                .expect("failed to get device format");

            device
                .open_device(format)
                .expect("failed to open device with default format")
        };

        self.device = Some(device);
        self.stream = Some(stream);

        let format = self.stream.as_mut().unwrap().get_current_format().unwrap();

        info!(
            "Opened device: {:?}, format: {:?}, rate: {}, channel_count: {}",
            self.device.as_ref().unwrap().get_name(),
            format.sample_type,
            format.sample_rate,
            format.channels.count()
        );
    }

    /// Uses the current media provider to decode audio samples and sends them to the current
    /// playback stream.
    fn play_audio(&mut self) {
        let Some(stream) = &mut self.stream else {
            return;
        };
        let Some(provider) = &mut self.media_provider else {
            return;
        };
        if self.resampler.is_none() {
            // TODO: proper error handling
            // Read the first samples ahead of time to determine the format.
            let first_samples = match provider.read_samples() {
                Ok(samples) => samples,
                Err(e) => match e {
                    PlaybackReadError::NothingOpen => {
                        panic!("thread state is invalid: no file open")
                    }
                    PlaybackReadError::NeverStarted => {
                        panic!("thread state is invalid: playback never started")
                    }
                    PlaybackReadError::Eof => {
                        info!("EOF, moving to next song");
                        self.next(false);
                        return;
                    }
                    PlaybackReadError::Unknown(s) => {
                        error!("unknown decode error: {}", s);
                        warn!("samples may be skipped");
                        return;
                    }
                    PlaybackReadError::DecodeFatal(s) => {
                        error!("fatal decoding error: {}, moving to next song", s);
                        self.next(false);
                        return;
                    }
                },
            };

            // Set up the resampler
            let duration = provider.frame_duration().expect("can't get duration");
            let device_format = stream.get_current_format().unwrap();

            let resampler_sample_rate =
                (device_format.sample_rate / device_format.rate_channel_ratio as u32) * 2;

            self.resampler = Some(Resampler::new(
                first_samples.rate,
                resampler_sample_rate,
                duration,
                device_format.channels.count(),
            ));
            self.format = Some(device_format.clone());

            // Convert the first samples to the device format
            let converted = self
                .resampler
                .as_mut()
                .unwrap()
                .convert_formats(first_samples, self.format.as_ref().unwrap());

            // Submit the converted samples to the stream
            let submit_frame = stream.submit_frame(converted.clone());

            // If we get an error, recreate the stream and retry
            if submit_frame.is_err() {
                let format = self.format.clone();
                warn!(
                    "Failed to submit frame, recreating device and retrying... {:?}",
                    submit_frame.err().unwrap()
                );
                self.recreate_stream(true, format.map(|v| v.channels));
                let final_result = self.stream.as_mut().unwrap().submit_frame(converted);

                if final_result.is_err() {
                    error!("Failed to submit frame after recreation");
                    error!("This likely indicates a problem with the audio device or driver");
                    error!("(or an underlying issue in the used DeviceProvider)");
                    error!("Please check your audio setup and try again.");
                    panic!("Failed to submit frame after recreation");
                }
            }

            self.update_ts();
        } else {
            // Ditto above but without creating the resampler
            let samples = match provider.read_samples() {
                Ok(samples) => samples,
                Err(e) => match e {
                    PlaybackReadError::NothingOpen => {
                        panic!("thread state is invalid: no file open")
                    }
                    PlaybackReadError::NeverStarted => {
                        panic!("thread state is invalid: playback never started")
                    }
                    PlaybackReadError::Eof => {
                        info!("EOF, moving to next song");
                        self.next(false);
                        return;
                    }
                    PlaybackReadError::Unknown(s) => {
                        error!("unknown decode error: {}", s);
                        warn!("samples may be skipped");
                        return;
                    }
                    PlaybackReadError::DecodeFatal(s) => {
                        error!("fatal decoding error: {}, moving to next song", s);
                        self.next(false);
                        return;
                    }
                },
            };
            let converted = self
                .resampler
                .as_mut()
                .unwrap()
                .convert_formats(samples, self.format.as_ref().unwrap());

            debug!("Submitting frame");
            let submit_frame = stream.submit_frame(converted.clone());
            debug!("Finished submitting frame");

            // If we get an error, recreate the stream and retry
            if submit_frame.is_err() {
                debug!("Submission error");
                let format = self.format.clone();
                warn!(
                    "Failed to submit frame, recreating device and retrying... {:?}",
                    submit_frame.err().unwrap()
                );
                self.recreate_stream(true, format.map(|v| v.channels));
                let final_result = self.stream.as_mut().unwrap().submit_frame(converted);

                if final_result.is_err() {
                    error!("Failed to submit frame after recreation");
                    error!("This likely indicates a problem with the audio device or driver");
                    error!("(or an underlying issue in the used DeviceProvider)");
                    error!("Please check your audio setup and try again.");
                    panic!("Failed to submit frame after recreation");
                }
            }

            self.update_ts();
        }
    }
}
