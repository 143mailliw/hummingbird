use std::{ffi::c_void, path::Path, sync::Arc, time::Duration};

use async_lock::Mutex;
use async_trait::async_trait;
use raw_window_handle::RawWindowHandle;
use windows::{
    core::HSTRING,
    Foundation::TypedEventHandler,
    Media::{
        MediaPlaybackAutoRepeatMode, MediaPlaybackStatus, MediaPlaybackType,
        SystemMediaTransportControls, SystemMediaTransportControlsButton,
        SystemMediaTransportControlsButtonPressedEventArgs,
        SystemMediaTransportControlsDisplayUpdater, SystemMediaTransportControlsTimelineProperties,
    },
    Storage::Streams::{DataWriter, InMemoryRandomAccessStream, RandomAccessStreamReference},
    Win32::{Foundation::HWND, System::WinRT::ISystemMediaTransportControlsInterop},
};

use crate::{
    media::metadata::Metadata,
    playback::{events::RepeatState, thread::PlaybackState},
    services::controllers::InitPlaybackController,
};

use super::{ControllerBridge, PlaybackController};

pub struct WindowsController {
    controls: SystemMediaTransportControls,
    display: SystemMediaTransportControlsDisplayUpdater,
    timeline: SystemMediaTransportControlsTimelineProperties,
    bridge: ControllerBridge,
}

impl WindowsController {
    pub fn connect_events(&mut self) {
        self.controls
            .SetIsEnabled(true)
            .expect("could not enable SMTC");
        self.controls
            .SetIsNextEnabled(true)
            .expect("could not enable SMTC");
        self.controls
            .SetIsPreviousEnabled(true)
            .expect("could not enable SMTC");
        self.controls
            .SetIsPlayEnabled(true)
            .expect("could not enable SMTC");
        self.controls
            .SetIsPauseEnabled(true)
            .expect("could not enable SMTC");

        let bridge = self.bridge.clone();
        self.controls
            .ButtonPressed(&TypedEventHandler::<
                SystemMediaTransportControls,
                SystemMediaTransportControlsButtonPressedEventArgs,
            >::new(move |_, args| {
                let event = args.as_ref().unwrap().Button().unwrap();

                match event {
                    SystemMediaTransportControlsButton::Play => bridge.play(),
                    SystemMediaTransportControlsButton::Pause => bridge.pause(),
                    SystemMediaTransportControlsButton::Next => bridge.next(),
                    SystemMediaTransportControlsButton::Previous => bridge.previous(),
                    _ => (),
                }

                Ok(())
            }))
            .expect("could not register button handler");
    }
}

impl InitPlaybackController for WindowsController {
    fn init(
        bridge: ControllerBridge,
        handle: Option<RawWindowHandle>,
    ) -> Arc<Mutex<dyn PlaybackController>> {
        let interop: ISystemMediaTransportControlsInterop = windows::core::factory::<
            SystemMediaTransportControls,
            ISystemMediaTransportControlsInterop,
        >()
        .expect("failed to create SMTC factory");

        let hwnd = match handle {
            Some(RawWindowHandle::Win32(handle)) => handle,
            _ => panic!("non-Win32 window handle/invalid window handle during creation of SMTC"),
        };

        let controls: SystemMediaTransportControls = unsafe {
            let pointer = hwnd.hwnd.get() as *mut c_void;
            interop.GetForWindow(HWND(pointer)).unwrap()
        };

        let display = controls.DisplayUpdater().unwrap();
        let timeline = SystemMediaTransportControlsTimelineProperties::new().unwrap();

        let mut controller = WindowsController {
            controls,
            display,
            timeline,
            bridge,
        };

        controller.connect_events();

        Arc::new(Mutex::new(controller))
    }
}

#[async_trait]
impl PlaybackController for WindowsController {
    async fn position_changed(&mut self, new_position: u64) {
        self.timeline
            .SetPosition(Duration::from_secs(new_position).into())
            .expect("could not set_position");
        self.controls
            .UpdateTimelineProperties(&self.timeline)
            .expect("could not update timeline");
    }
    async fn duration_changed(&mut self, new_duration: u64) {
        self.timeline
            .SetStartTime(Duration::from_secs(0).into())
            .expect("could not set start time");
        self.timeline
            .SetMinSeekTime(Duration::from_secs(0).into())
            .expect("could not set min seek time");
        self.timeline
            .SetMaxSeekTime(Duration::from_secs(new_duration).into())
            .expect("could not set max seek time");
        self.timeline
            .SetEndTime(Duration::from_secs(new_duration).into())
            .expect("could not set duration");
        self.timeline
            .SetPosition(Duration::from_secs(0).into())
            .expect("could not set position");
        self.controls
            .UpdateTimelineProperties(&self.timeline)
            .expect("could not update timeline");
    }
    async fn volume_changed(&mut self, _new_volume: f64) {}
    async fn metadata_changed(&mut self, metadata: &Metadata) {
        if let Some(title) = metadata.name.clone() {
            let string = HSTRING::from(title);
            self.display
                .MusicProperties()
                .unwrap()
                .SetTitle(&string)
                .expect("could not set title");
        }

        if let Some(artist) = metadata.artist.clone() {
            let string = HSTRING::from(artist);
            self.display
                .MusicProperties()
                .unwrap()
                .SetArtist(&string)
                .expect("could not set artist");
        }

        if let Some(album) = metadata.album.clone() {
            let string = HSTRING::from(album);
            self.display
                .MusicProperties()
                .unwrap()
                .SetAlbumTitle(&string)
                .expect("could not set album");
        }

        if let Some(track_number) = metadata.track_current {
            self.display
                .MusicProperties()
                .unwrap()
                .SetTrackNumber(track_number as u32)
                .expect("could not set track number");
        }

        if let Some(track_max) = metadata.track_max {
            self.display
                .MusicProperties()
                .unwrap()
                .SetAlbumTrackCount(track_max as u32)
                .expect("could not set track number");
        }

        self.display.Update().expect("could not update");
    }
    async fn album_art_changed(&mut self, album_art: &[u8]) {
        let stream = InMemoryRandomAccessStream::new().expect("could not create RAS");
        let writer = DataWriter::CreateDataWriter(&stream).unwrap();

        writer
            .WriteBytes(album_art)
            .expect("could not start writing operation");

        writer
            .StoreAsync()
            .expect("could not start store operation")
            .await
            .expect("could not complete store operation");

        writer
            .DetachStream()
            .expect("could not detach writer from stream");

        let reference = RandomAccessStreamReference::CreateFromStream(&stream)
            .expect("could not create stream reference");

        self.display
            .SetThumbnail(&reference)
            .expect("could not set thumbnail reference");

        self.display.Update().expect("could not update");
    }
    async fn repeat_state_changed(&mut self, repeat_state: RepeatState) {
        self.controls
            .SetAutoRepeatMode(match repeat_state {
                RepeatState::NotRepeating => MediaPlaybackAutoRepeatMode::None,
                RepeatState::Repeating => MediaPlaybackAutoRepeatMode::List,
                RepeatState::RepeatingOne => MediaPlaybackAutoRepeatMode::Track,
            })
            .expect("could not set auto repeat mode");
    }
    async fn playback_state_changed(&mut self, playback_state: PlaybackState) {
        let playback_state = match playback_state {
            PlaybackState::Stopped => MediaPlaybackStatus::Stopped,
            PlaybackState::Playing => MediaPlaybackStatus::Playing,
            PlaybackState::Paused => MediaPlaybackStatus::Paused,
        };

        self.controls
            .SetPlaybackStatus(playback_state)
            .expect("could not set playback status");
    }
    async fn shuffle_state_changed(&mut self, shuffling: bool) {
        self.controls
            .SetShuffleEnabled(shuffling)
            .expect("could not set shuffle status");
    }
    async fn new_file(&mut self, path: &Path) {
        self.display.ClearAll().expect("could not clear display");
        self.display.SetType(MediaPlaybackType::Music).unwrap();
        let title_string = HSTRING::from(path.file_name().unwrap().to_str().unwrap());
        self.display
            .MusicProperties()
            .unwrap()
            .SetTitle(&title_string)
            .expect("could not set initial title");
        self.display.Update().expect("could not update display");
    }
}
