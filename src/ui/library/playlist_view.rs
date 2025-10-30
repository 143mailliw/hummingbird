use std::sync::Arc;

use ahash::AHashMap;
use gpui::{
    App, AppContext, Context, Entity, FontWeight, ParentElement, Render, Styled, Window, div, px,
    rems, uniform_list,
};

use crate::{
    library::{
        db::LibraryAccess,
        types::{Playlist, PlaylistType},
    },
    playback::{
        interface::{GPUIPlaybackInterface, replace_queue},
        queue::QueueItemData,
    },
    ui::{
        components::{
            button::{ButtonIntent, ButtonSize, button},
            icons::{CIRCLE_PLUS, PLAY, PLAYLIST, SHUFFLE, STAR, icon},
        },
        library::track_listing::{
            ArtistNameVisibility,
            track_item::{TrackItem, TrackItemLeftField},
        },
        models::{Models, PlaybackInfo, PlaylistEvent},
        theme::Theme,
        util::{create_or_retrieve_view, prune_views},
    },
};

use super::track_listing::track_item::TrackPlaylistInfo;

pub struct PlaylistView {
    playlist: Arc<Playlist>,
    playlist_track_ids: Arc<Vec<(i64, i64, i64)>>,
    views: Entity<AHashMap<usize, Entity<TrackItem>>>,
    render_counter: Entity<usize>,
}

impl PlaylistView {
    pub fn new(cx: &mut App, playlist_id: i64) -> Entity<Self> {
        cx.new(|cx| {
            let playlist_tracker = cx.global::<Models>().playlist_tracker.clone();

            cx.subscribe(
                &playlist_tracker,
                |this: &mut Self, _, ev: &PlaylistEvent, cx| match ev {
                    PlaylistEvent::PlaylistUpdated(id) => {
                        if *id == this.playlist.id {
                            this.playlist_track_ids =
                                cx.get_playlist_tracks(this.playlist.id).unwrap();
                        }
                    }
                },
            )
            .detach();

            Self {
                playlist: cx.get_playlist(playlist_id).unwrap(),
                playlist_track_ids: cx.get_playlist_tracks(playlist_id).unwrap(),
                views: cx.new(|_| AHashMap::new()),
                render_counter: cx.new(|_| 0),
            }
        })
    }
}

impl Render for PlaylistView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.global::<Theme>();
        let items_clone = self.playlist_track_ids.clone();
        let views_model = self.views.clone();
        let render_counter = self.render_counter.clone();
        let pl_id = self.playlist.id;

        div()
            .pt(px(10.0))
            .flex()
            .flex_col()
            .flex_shrink()
            .overflow_x_hidden()
            .max_w(px(1000.0))
            .h_full()
            .child(
                div()
                    .flex()
                    .overflow_x_hidden()
                    .flex_shrink()
                    .px(px(18.0))
                    .w_full()
                    .child(
                        div()
                            .bg(theme.album_art_background)
                            .shadow_sm()
                            .w(px(160.0))
                            .h(px(160.0))
                            .flex_shrink_0()
                            .rounded(px(4.0))
                            .overflow_hidden()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                icon(if self.playlist.playlist_type == PlaylistType::System {
                                    STAR
                                } else {
                                    PLAYLIST
                                })
                                .size(px(100.0)),
                            ),
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
                            .child(
                                div()
                                    .font_weight(FontWeight::EXTRA_BOLD)
                                    .text_size(rems(2.5))
                                    .line_height(rems(2.75))
                                    .overflow_x_hidden()
                                    .pb(px(10.0))
                                    .w_full()
                                    .text_ellipsis()
                                    .child(self.playlist.name.clone()),
                            )
                            .child(
                                div()
                                    .gap(px(10.0))
                                    .flex()
                                    .child(
                                        button()
                                            .id("playlist-play-button")
                                            .size(ButtonSize::Large)
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .intent(ButtonIntent::Primary)
                                            .child(icon(PLAY).size(px(16.0)).my_auto())
                                            .child("Play")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let tracks = cx
                                                    .get_playlist_track_files(this.playlist.id)
                                                    .unwrap();

                                                let queue_items = this
                                                    .playlist_track_ids
                                                    .iter()
                                                    .zip(tracks.iter())
                                                    .map(|((_, track, album), path)| {
                                                        QueueItemData::new(
                                                            cx,
                                                            path.into(),
                                                            Some(*track),
                                                            Some(*album),
                                                        )
                                                    })
                                                    .collect();

                                                replace_queue(queue_items, cx);
                                            })),
                                    )
                                    .child(
                                        button()
                                            .id("playlist-add-button")
                                            .size(ButtonSize::Large)
                                            .flex_none()
                                            .child(icon(CIRCLE_PLUS).size(px(16.0)).my_auto())
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let tracks = cx
                                                    .get_playlist_track_files(this.playlist.id)
                                                    .unwrap();

                                                let queue_items = this
                                                    .playlist_track_ids
                                                    .iter()
                                                    .zip(tracks.iter())
                                                    .map(|((_, track, album), path)| {
                                                        QueueItemData::new(
                                                            cx,
                                                            path.into(),
                                                            Some(*track),
                                                            Some(*album),
                                                        )
                                                    })
                                                    .collect();

                                                cx.global::<GPUIPlaybackInterface>()
                                                    .queue_list(queue_items);
                                            })),
                                    )
                                    .child(
                                        button()
                                            .id("playlist-shuffle-button")
                                            .size(ButtonSize::Large)
                                            .flex_none()
                                            .child(icon(SHUFFLE).size(px(16.0)).my_auto())
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let tracks = cx
                                                    .get_playlist_track_files(this.playlist.id)
                                                    .unwrap();

                                                let queue_items = this
                                                    .playlist_track_ids
                                                    .iter()
                                                    .zip(tracks.iter())
                                                    .map(|((_, track, album), path)| {
                                                        QueueItemData::new(
                                                            cx,
                                                            path.into(),
                                                            Some(*track),
                                                            Some(*album),
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

                                                replace_queue(queue_items, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(
                uniform_list("playlist-list", items_clone.len(), move |range, _, cx| {
                    let start = range.start;
                    let is_templ_render = range.start == 0 && range.end == 1;

                    let items = &items_clone[range];

                    items
                        .iter()
                        .enumerate()
                        .map(|(idx, item)| {
                            let idx = idx + start;

                            if !is_templ_render {
                                prune_views(&views_model, &render_counter, idx, cx);
                            }

                            div().child(create_or_retrieve_view(
                                &views_model,
                                idx,
                                move |cx| {
                                    let track = cx.get_track_by_id(item.1).unwrap();
                                    TrackItem::new(
                                        cx,
                                        Arc::try_unwrap(track).unwrap(),
                                        false,
                                        ArtistNameVisibility::Always,
                                        TrackItemLeftField::Art,
                                        Some(TrackPlaylistInfo {
                                            id: pl_id,
                                            item_id: item.0,
                                        }),
                                    )
                                },
                                cx,
                            ))
                        })
                        .collect()
                })
                .w_full()
                .h_full()
                .flex()
                .flex_col()
                .border_color(theme.border_color)
                .border_t_1()
                .mt(px(18.0)),
            )
    }
}
