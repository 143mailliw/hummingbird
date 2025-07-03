use std::sync::Arc;

use gpui::*;
use prelude::FluentBuilder;

use crate::{
    library::{
        db::{AlbumMethod, LibraryAccess},
        types::{Album, Artist, Track},
    },
    playback::{
        interface::{replace_queue, GPUIPlaybackInterface},
        queue::QueueItemData,
        thread::PlaybackState,
    },
    ui::{
        components::button::{button, ButtonIntent, ButtonSize},
        constants::FONT_AWESOME,
        global_actions::PlayPause,
        library::track_listing::TrackListing,
        models::PlaybackInfo,
        theme::Theme,
    },
};

use super::track_listing::ArtistNameVisibility;

pub struct ReleaseView {
    album: Arc<Album>,
    artist: Option<Arc<Artist>>,
    tracks: Arc<Vec<Track>>,
    track_listing: TrackListing,
    release_info: Option<SharedString>,
    img_path: SharedString,
    image_cache: Entity<RetainAllImageCache>,
}

impl ReleaseView {
    pub(super) fn new(cx: &mut App, album_id: i64) -> Entity<Self> {
        cx.new(|cx| {
            let image_cache = RetainAllImageCache::new(cx);
            // TODO: error handling
            let album = cx
                .get_album_by_id(album_id, AlbumMethod::FullQuality)
                .expect("Failed to retrieve album");
            let tracks = cx
                .list_tracks_in_album(album_id)
                .expect("Failed to retrieve tracks");
            let artist = cx.get_artist_by_id(album.artist_id).ok();

            cx.on_release(|this: &mut Self, cx: &mut App| {
                ImageSource::Resource(Resource::Embedded(this.img_path.clone())).remove_asset(cx);
            })
            .detach();

            let track_listing = TrackListing::new(
                tracks.clone(),
                ArtistNameVisibility::OnlyIfDifferent(artist.as_ref().and_then(|v| v.name.clone())),
            );

            let release_info = {
                let mut info = String::default();

                if let Some(label) = &album.label {
                    info += &label.to_string();
                }

                if album.label.is_some() && album.catalog_number.is_some() {
                    info += " • ";
                }

                if let Some(catalog_number) = &album.catalog_number {
                    info += &catalog_number.to_string();
                }

                if !info.is_empty() {
                    Some(SharedString::from(info))
                } else {
                    None
                }
            };

            ReleaseView {
                album,
                artist,
                tracks,
                track_listing,
                release_info,
                img_path: SharedString::from(format!("!db://album/{album_id}/full")),
                image_cache,
            }
        })
    }
}

impl Render for ReleaseView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let is_playing =
            cx.global::<PlaybackInfo>().playback_state.read(cx) == &PlaybackState::Playing;
        // flag whether current track is part of the album
        let current_track_in_album = cx
            .global::<PlaybackInfo>()
            .current_track
            .read(cx)
            .clone()
            .is_some_and(|current_track| {
                self.tracks
                    .iter()
                    .any(|track| current_track == track.location)
            });

        div()
            .mt(px(24.0))
            .w_full()
            .flex_shrink()
            .overflow_x_hidden()
            .h_full()
            .max_w(px(1000.0))
            .mx_auto()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_shrink()
                    .flex()
                    .overflow_x_hidden()
                    .px(px(24.0))
                    .w_full()
                    .child(
                        div()
                            .rounded(px(4.0))
                            .bg(theme.album_art_background)
                            .shadow_sm()
                            .w(px(160.0))
                            .h(px(160.0))
                            .flex_shrink_0()
                            .overflow_hidden()
                            .child(
                                img(self.img_path.clone())
                                    .image_cache(&self.image_cache)
                                    .min_w(px(160.0))
                                    .min_h(px(160.0))
                                    .max_w(px(160.0))
                                    .max_h(px(160.0))
                                    .overflow_hidden()
                                    .flex()
                                    // TODO: Ideally this should be ObjectFit::Cover, but for
                                    // some reason that makes the element bigger
                                    // FIXME: Is this a GPUI bug?
                                    .object_fit(ObjectFit::Fill)
                                    .rounded(px(4.0)),
                            ), // .when(image.is_some(), |div| {
                               //     div.child(
                               //         img(image.clone().unwrap())
                               //             .min_w(px(160.0))
                               //             .min_h(px(160.0))
                               //             .max_w(px(160.0))
                               //             .max_h(px(160.0))
                               //             .overflow_hidden()
                               //             .flex()
                               //             // TODO: Ideally this should be ObjectFit::Cover, but for
                               //             // some reason that makes the element bigger
                               //             // FIXME: Is this a GPUI bug?
                               //             .object_fit(ObjectFit::Fill)
                               //             .rounded(px(4.0)),
                               //     )
                               // }),
                    )
                    .child(
                        div()
                            .ml(px(18.0))
                            .mt_auto()
                            .flex_shrink()
                            .flex()
                            .flex_col()
                            .w_full()
                            .overflow_x_hidden()
                            .child(div().when_some(
                                self.artist.as_ref().map(|v| v.name.clone()),
                                |this, artist| this.child(artist.unwrap()),
                            ))
                            .child(
                                div()
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .text_size(rems(2.5))
                                    .line_height(rems(2.75))
                                    .overflow_x_hidden()
                                    .pb(px(10.0))
                                    .min_w_0()
                                    .text_ellipsis()
                                    .child(self.album.title.clone()),
                            )
                            .child(
                                div()
                                    .gap(px(10.0))
                                    .flex()
                                    .flex_row()
                                    .child(
                                        button()
                                            .id("release-play-button")
                                            .size(ButtonSize::Large)
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .intent(ButtonIntent::Primary)
                                            .when(!current_track_in_album, |this| {
                                                this.on_click(cx.listener(
                                                    |this: &mut ReleaseView, _, _, cx| {
                                                        let queue_items = this
                                                            .track_listing
                                                            .tracks()
                                                            .iter()
                                                            .map(|track| {
                                                                QueueItemData::new(
                                                                    cx,
                                                                    track.location.clone(),
                                                                    Some(track.id),
                                                                    track.album_id,
                                                                )
                                                            })
                                                            .collect();

                                                        replace_queue(queue_items, cx)
                                                    },
                                                ))
                                            })
                                            .when(current_track_in_album, |button| {
                                                button.on_click(|_, window, cx| {
                                                    window.dispatch_action(Box::new(PlayPause), cx);
                                                })
                                            })
                                            .child(div().font_family(FONT_AWESOME).child(
                                                if current_track_in_album && is_playing {
                                                    ""
                                                } else {
                                                    ""
                                                },
                                            ))
                                            .child(div().child(
                                                if current_track_in_album && is_playing {
                                                    "Pause"
                                                } else {
                                                    "Play"
                                                },
                                            )),
                                    )
                                    .child(
                                        button()
                                            .id("release-add-button")
                                            .size(ButtonSize::Large)
                                            .flex_none()
                                            .on_click(cx.listener(
                                                |this: &mut ReleaseView, _, _, cx| {
                                                    let queue_items = this
                                                        .track_listing
                                                        .tracks()
                                                        .iter()
                                                        .map(|track| {
                                                            QueueItemData::new(
                                                                cx,
                                                                track.location.clone(),
                                                                Some(track.id),
                                                                track.album_id,
                                                            )
                                                        })
                                                        .collect();

                                                    cx.global::<GPUIPlaybackInterface>()
                                                        .queue_list(queue_items);
                                                },
                                            ))
                                            .child(div().font_family(FONT_AWESOME).child("")),
                                    )
                                    .child(
                                        button()
                                            .id("release-shuffle-button")
                                            .size(ButtonSize::Large)
                                            .flex_none()
                                            .on_click(cx.listener(
                                                |this: &mut ReleaseView, _, _, cx| {
                                                    let queue_items = this
                                                        .track_listing
                                                        .tracks()
                                                        .iter()
                                                        .map(|track| {
                                                            QueueItemData::new(
                                                                cx,
                                                                track.location.clone(),
                                                                Some(track.id),
                                                                track.album_id,
                                                            )
                                                        })
                                                        .collect();

                                                    if !(*cx
                                                        .global::<PlaybackInfo>()
                                                        .shuffling
                                                        .read(cx))
                                                    {
                                                        cx.global::<GPUIPlaybackInterface>()
                                                            .toggle_shuffle();
                                                    }

                                                    replace_queue(queue_items, cx)
                                                },
                                            ))
                                            .child(div().font_family(FONT_AWESOME).child("")),
                                    ),
                            ),
                    ),
            )
            .child(
                list(self.track_listing.track_list_state().clone())
                    .w_full()
                    .flex()
                    .h_full()
                    .flex_col()
                    .mx_auto(),
            )
            .when(
                self.release_info.is_some()
                    || self.album.release_date.is_some()
                    || self.album.isrc.is_some(),
                |this| {
                    this.child(
                        div()
                            .flex()
                            .flex_col()
                            .text_sm()
                            .ml(px(24.0))
                            .pt(px(12.0))
                            .pb(px(24.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(theme.text_secondary)
                            .when_some(self.release_info.clone(), |this, release_info| {
                                this.child(div().child(release_info))
                            })
                            .when_some(self.album.release_date, |this, date| {
                                this.child(
                                    div().child(format!("Released {}", date.format("%B %-e, %Y"))),
                                )
                            })
                            .when_some(self.album.isrc.as_ref(), |this, isrc| {
                                this.child(div().child(isrc.clone()))
                            }),
                    )
                },
            )
    }
}
