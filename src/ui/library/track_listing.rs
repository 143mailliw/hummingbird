mod track_item;

use std::sync::Arc;

use gpui::{px, IntoElement, ListAlignment, ListState};

use crate::library::types::{DBString, Track};
use track_item::TrackItem;

#[derive(Clone, Debug, PartialEq)]
pub enum ArtistNameVisibility {
    Always,
    Never,
    OnlyIfDifferent(Option<DBString>),
}

#[derive(Clone)]
pub struct TrackListing {
    tracks: Arc<Vec<PrimitiveTrack>>,
    track_list_state: ListState,
}

impl TrackListing {
    pub fn new(tracks: Arc<Vec<Track>>, artist_name_visibility: ArtistNameVisibility) -> Self {
        let tracks_clone = tracks.clone();
        let state = ListState::new(
            tracks.len(),
            ListAlignment::Top,
            px(25.0),
            move |idx, _, _| {
                TrackItem {
                    track: tracks_clone[idx].clone(),
                    is_start: if idx > 0 {
                        if let Some(track) = tracks_clone.get(idx - 1) {
                            track.disc_number != tracks_clone[idx].disc_number
                        } else {
                            tracks_clone[idx].disc_number >= Some(0)
                        }
                    } else {
                        true
                    },
                    artist_name_visibility: artist_name_visibility.clone(),
                }
                .into_any_element()
            },
        );

        Self {
            tracks,
            track_list_state: state,
        }
    }

    pub fn tracks(&self) -> &Arc<Vec<Track>> {
        &self.tracks
    }

    pub fn track_list_state(&self) -> &ListState {
        &self.track_list_state
    }
}
