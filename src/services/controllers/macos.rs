use std::{io::Cursor, path::Path, ptr::NonNull, sync::Arc};

use async_lock::Mutex;
use async_trait::async_trait;
use block2::{Block, RcBlock};
use objc2::{class, msg_send, rc::Retained, runtime::ProtocolObject, AnyThread};
use objc2_app_kit::NSImage;
use objc2_core_foundation::CGSize;
use objc2_foundation::{ns_string, NSData, NSMutableDictionary, NSNumber, NSString};
use objc2_media_player::{
    MPMediaItemArtwork, MPMediaItemPropertyAlbumTitle, MPMediaItemPropertyArtist,
    MPMediaItemPropertyArtwork, MPMediaItemPropertyPlaybackDuration, MPMediaItemPropertyTitle,
    MPNowPlayingInfoCenter, MPNowPlayingInfoPropertyElapsedPlaybackTime, MPNowPlayingPlaybackState,
};
use tracing::{debug, info};

use crate::{
    media::metadata::Metadata,
    playback::{
        events::{PlaybackCommand, RepeatState},
        thread::PlaybackState,
    },
};

use super::{ControllerBridge, InitPlaybackController, PlaybackController};

pub struct MacMediaPlayerController {
    bridge: ControllerBridge,
}

impl MacMediaPlayerController {
    unsafe fn new_file(&mut self, path: &Path) {
        info!("New file: {:?}", path);

        let file_name = path
            .file_name()
            .expect("files should have file names")
            .to_str()
            .expect("files should have UTF-8 names");

        let media_center = MPNowPlayingInfoCenter::defaultCenter();
        let now_playing: Retained<NSMutableDictionary<NSString>> =
            NSMutableDictionary::dictionary();

        let ns_name = NSString::from_str(file_name);
        now_playing.setObject_forKey(&ns_name, ProtocolObject::from_ref(MPMediaItemPropertyTitle));

        media_center.setNowPlayingInfo(Some(&*now_playing));
    }

    unsafe fn new_metadata(&mut self, metadata: &Metadata) {
        let media_center = MPNowPlayingInfoCenter::defaultCenter();
        let now_playing: Retained<NSMutableDictionary<NSString>> =
            NSMutableDictionary::dictionary();

        if let Some(prev_now_playing) = media_center.nowPlayingInfo() {
            now_playing.addEntriesFromDictionary(&prev_now_playing);
        }

        if let Some(title) = &metadata.name {
            info!("Setting title: {}", title);
            let ns = NSString::from_str(title);
            now_playing.setObject_forKey(&ns, ProtocolObject::from_ref(MPMediaItemPropertyTitle));
        }

        if let Some(artist) = &metadata.artist {
            info!("Setting artist: {}", artist);
            let ns = NSString::from_str(artist);
            now_playing.setObject_forKey(&ns, ProtocolObject::from_ref(MPMediaItemPropertyArtist));
        }

        if let Some(album_title) = &metadata.album {
            info!("Setting album title: {}", album_title);
            let ns = NSString::from_str(album_title);
            now_playing
                .setObject_forKey(&ns, ProtocolObject::from_ref(MPMediaItemPropertyAlbumTitle));
        }

        media_center.setNowPlayingInfo(Some(&*now_playing));
    }

    unsafe fn new_duration(&mut self, duration: u64) {
        let media_center = MPNowPlayingInfoCenter::defaultCenter();
        let now_playing: Retained<NSMutableDictionary<NSString>> =
            NSMutableDictionary::dictionary();

        if let Some(prev_now_playing) = media_center.nowPlayingInfo() {
            now_playing.addEntriesFromDictionary(&prev_now_playing);
        }

        let ns = NSNumber::numberWithUnsignedLong(duration);
        now_playing.setObject_forKey(
            &ns,
            ProtocolObject::from_ref(MPMediaItemPropertyPlaybackDuration),
        );

        media_center.setNowPlayingInfo(Some(&*now_playing));
    }

    unsafe fn new_position(&mut self, position: u64) {
        let media_center = MPNowPlayingInfoCenter::defaultCenter();
        let now_playing: Retained<NSMutableDictionary<NSString>> =
            NSMutableDictionary::dictionary();

        if let Some(prev_now_playing) = media_center.nowPlayingInfo() {
            now_playing.addEntriesFromDictionary(&prev_now_playing);
        }

        let ns = NSNumber::numberWithUnsignedLong(position);
        now_playing.setObject_forKey(
            &ns,
            ProtocolObject::from_ref(MPNowPlayingInfoPropertyElapsedPlaybackTime),
        );

        media_center.setNowPlayingInfo(Some(&*now_playing));
    }

    unsafe fn new_album_art(&mut self, art: &[u8]) {
        // get the image's dimensions, we'll need them to load the image into NP
        let Ok(size) = imagesize::blob_size(art) else {
            return;
        };

        let data = NSData::with_bytes(art);
        let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) else {
            return;
        };
        // there's a good chance this leaks memory
        // the only way that it wouldn't is if, once it disappears in to macOS, the OS drops it
        // there's an even better chance that if it does, there's no way to fix it
        // TODO: figure out this mess
        let image = NonNull::new(Retained::into_raw(image)).unwrap();

        let request_handler = RcBlock::new(move |_cg: CGSize| image);
        let bounds_size = CGSize::new(size.width as f64, size.height as f64);
        let artwork = MPMediaItemArtwork::initWithBoundsSize_requestHandler(
            MPMediaItemArtwork::alloc(),
            bounds_size,
            &request_handler,
        );

        let media_center = MPNowPlayingInfoCenter::defaultCenter();
        let now_playing: Retained<NSMutableDictionary<NSString>> =
            NSMutableDictionary::dictionary();

        if let Some(prev_now_playing) = media_center.nowPlayingInfo() {
            now_playing.addEntriesFromDictionary(&prev_now_playing);
        }

        now_playing.setObject_forKey(
            &artwork,
            ProtocolObject::from_ref(MPMediaItemPropertyArtwork),
        );
    }

    unsafe fn new_playback_state(&mut self, state: PlaybackState) {
        info!("Setting playback state: {:?}", state);
        let media_center = MPNowPlayingInfoCenter::defaultCenter();
        media_center.setPlaybackState(match state {
            PlaybackState::Stopped => MPNowPlayingPlaybackState::Stopped,
            PlaybackState::Playing => MPNowPlayingPlaybackState::Playing,
            PlaybackState::Paused => MPNowPlayingPlaybackState::Paused,
        });
    }
}

#[async_trait]
impl PlaybackController for MacMediaPlayerController {
    async fn position_changed(&mut self, new_position: u64) {
        unsafe { self.new_position(new_position) }
    }
    async fn duration_changed(&mut self, new_duration: u64) {
        unsafe { self.new_duration(new_duration) }
    }
    async fn volume_changed(&mut self, new_volume: f64) {
        ()
    }
    async fn metadata_changed(&mut self, metadata: &Metadata) {
        unsafe { self.new_metadata(metadata) }
    }
    async fn album_art_changed(&mut self, album_art: &[u8]) {
        unsafe { self.new_album_art(album_art) }
    }
    async fn repeat_state_changed(&mut self, repeat_state: RepeatState) {
        ()
    }
    async fn playback_state_changed(&mut self, playback_state: PlaybackState) {
        unsafe { self.new_playback_state(playback_state) }
    }
    async fn new_file(&mut self, path: &Path) {
        unsafe { self.new_file(path) }
    }
}

impl InitPlaybackController for MacMediaPlayerController {
    fn init(bridge: ControllerBridge) -> Arc<Mutex<dyn PlaybackController>> {
        Arc::new(Mutex::new(MacMediaPlayerController { bridge }))
    }
}
