use gpui::prelude::{FluentBuilder, *};
use gpui::{App, Entity, FontWeight, IntoElement, SharedString, Window, div, img, px};

use crate::ui::components::icons::{PLAY, PLUS, STAR, STAR_FILLED, icon};
use crate::ui::models::PlaylistEvent;
use crate::{
    library::{db::LibraryAccess, types::Track},
    playback::{
        interface::{GPUIPlaybackInterface, replace_queue},
        queue::QueueItemData,
    },
    ui::{
        components::{
            context::context,
            menu::{menu, menu_item},
        },
        models::{Models, PlaybackInfo},
        theme::Theme,
    },
};

use super::ArtistNameVisibility;

pub struct TrackPlaylistInfo {
    pub id: i64,
    pub item_id: i64,
}

pub struct TrackItem {
    pub track: Track,
    pub is_start: bool,
    pub artist_name_visibility: ArtistNameVisibility,
    pub is_liked: Option<i64>,
    pub hover_group: SharedString,
    left_field: TrackItemLeftField,
    album_art: Option<SharedString>,
    pl_info: Option<TrackPlaylistInfo>,
}

#[derive(Eq, PartialEq)]
pub enum TrackItemLeftField {
    TrackNum,
    Art,
}

impl TrackItem {
    pub fn new(
        cx: &mut App,
        track: Track,
        is_start: bool,
        anv: ArtistNameVisibility,
        left_field: TrackItemLeftField,
        pl_info: Option<TrackPlaylistInfo>,
    ) -> Entity<Self> {
        cx.new(|cx| Self {
            hover_group: format!("track-{}", track.id).into(),
            is_liked: cx.playlist_has_track(1, track.id).unwrap_or_default(),
            album_art: track
                .album_id
                .map(|v| format!("!db://album/{v}/thumb").into()),
            track,
            is_start,
            artist_name_visibility: anv,
            left_field,
            pl_info,
        })
    }
}

impl Render for TrackItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let current_track = cx.global::<PlaybackInfo>().current_track.read(cx).clone();

        let track_location = self.track.location.clone();
        let track_location_2 = self.track.location.clone();
        let track_id = self.track.id;
        let album_id = self.track.album_id;

        let show_artist_name = self.artist_name_visibility != ArtistNameVisibility::Never
            && self.artist_name_visibility
                != ArtistNameVisibility::OnlyIfDifferent(self.track.artist_names.clone());

        let track = self.track.clone();

        context(("context", self.track.id as usize))
            .with(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .id(self.track.id as usize)
                    .on_click({
                        let track = self.track.clone();
                        let plid = self.pl_info.as_ref().map(|pl| pl.id);
                        move |_, _, cx| play_from_track(cx, &track, plid)
                    })
                    .when(self.is_start, |this| {
                        this.child(
                            div()
                                .text_color(theme.text_secondary)
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .px(px(18.0))
                                .border_b_1()
                                .w_full()
                                .border_color(theme.border_color)
                                .mt(px(24.0))
                                .pb(px(6.0))
                                .when_some(self.track.disc_number, |this, num| {
                                    this.child(format!("DISC {num}"))
                                }),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .border_b_1()
                            .id(("track", self.track.id as u64))
                            .w_full()
                            .border_color(theme.border_color)
                            .cursor_pointer()
                            .px(px(18.0))
                            .py(px(6.0))
                            .group(self.hover_group.clone())
                            .hover(|this| this.bg(theme.nav_button_hover))
                            .active(|this| this.bg(theme.nav_button_active))
                            .when_some(current_track, |this, track| {
                                this.bg(if track == self.track.location {
                                    theme.queue_item_current
                                } else {
                                    theme.background_primary
                                })
                            })
                            .max_w_full()
                            .when(self.left_field == TrackItemLeftField::TrackNum, |this| {
                                this.child(div().w(px(62.0)).flex_shrink_0().child(format!(
                                    "{}",
                                    self.track.track_number.unwrap_or_default()
                                )))
                            })
                            .when(self.left_field == TrackItemLeftField::Art, |this| {
                                this.child(
                                    div()
                                        .w(px(22.0))
                                        .h(px(22.0))
                                        .mr(px(12.0))
                                        .my_auto()
                                        .rounded(px(3.0))
                                        .bg(theme.album_art_background)
                                        .when_some(self.album_art.clone(), |this, art| {
                                            this.child(
                                                img(art).w(px(22.0)).h(px(22.0)).rounded(px(3.0)),
                                            )
                                        }),
                                )
                            })
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .overflow_x_hidden()
                                    .text_ellipsis()
                                    .child(self.track.title.clone()),
                            )
                            .child(
                                div()
                                    .id("like")
                                    .mr(px(-4.0))
                                    .ml_auto()
                                    .my_auto()
                                    .rounded_sm()
                                    .p(px(4.0))
                                    .child(
                                        icon(if self.is_liked.is_some() {
                                            STAR_FILLED
                                        } else {
                                            STAR
                                        })
                                        .size(px(14.0))
                                        .text_color(theme.text_secondary),
                                    )
                                    .invisible()
                                    .group(self.hover_group.clone())
                                    .group_hover(self.hover_group.clone(), |this| this.visible())
                                    .hover(|this| this.bg(theme.button_secondary_hover))
                                    .active(|this| this.bg(theme.button_secondary_active))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();

                                        if let Some(id) = this.is_liked {
                                            cx.remove_playlist_item(id)
                                                .expect("could not unlike song");

                                            this.is_liked = None;
                                        } else {
                                            this.is_liked = Some(
                                                cx.add_playlist_item(1, track_id)
                                                    .expect("could not like song"),
                                            );
                                        }

                                        let playlist_tracker =
                                            cx.global::<Models>().playlist_tracker.clone();

                                        playlist_tracker.update(cx, |_, cx| {
                                            cx.emit(PlaylistEvent::PlaylistUpdated(1));
                                        });

                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::LIGHT)
                                    .text_sm()
                                    .my_auto()
                                    .text_color(theme.text_secondary)
                                    .text_ellipsis()
                                    .overflow_x_hidden()
                                    .flex_shrink()
                                    .ml(px(12.0))
                                    .when(show_artist_name, |this| {
                                        this.when_some(
                                            self.track.artist_names.clone(),
                                            |this, v| this.child(v.0),
                                        )
                                    }),
                            )
                            .child(div().ml(px(12.0)).flex_shrink_0().child(format!(
                                "{}:{:02}",
                                self.track.duration / 60,
                                self.track.duration % 60
                            ))),
                    ),
            )
            .child(
                div().bg(theme.elevated_background).child(
                    menu()
                        .item(menu_item(
                            "track_play",
                            Some(PLAY),
                            "Play",
                            move |_, _, cx| {
                                let data = QueueItemData::new(
                                    cx,
                                    track_location.clone(),
                                    Some(track_id),
                                    album_id,
                                );
                                let playback_interface = cx.global::<GPUIPlaybackInterface>();
                                let queue_length = cx
                                    .global::<Models>()
                                    .queue
                                    .read(cx)
                                    .data
                                    .read()
                                    .expect("couldn't get queue")
                                    .len();
                                playback_interface.queue(data);
                                playback_interface.jump(queue_length);
                            },
                        ))
                        .item(menu_item(
                            "track_play_from_here",
                            None::<&str>,
                            "Play from here",
                            {
                                let plid = self.pl_info.as_ref().map(|pl| pl.id);
                                move |_, _, cx| play_from_track(cx, &track, plid)
                            },
                        ))
                        .item(menu_item(
                            "track_add_to_queue",
                            Some(PLUS),
                            "Add to queue",
                            move |_, _, cx| {
                                let data = QueueItemData::new(
                                    cx,
                                    track_location_2.clone(),
                                    Some(track_id),
                                    album_id,
                                );
                                let playback_interface = cx.global::<GPUIPlaybackInterface>();
                                playback_interface.queue(data);
                            },
                        )),
                ),
            )
    }
}

pub fn play_from_track(cx: &mut App, track: &Track, pl_id: Option<i64>) {
    let queue_items = if let Some(pl_id) = pl_id {
        let ids = cx
            .get_playlist_tracks(pl_id)
            .expect("failed to retrieve playlist track info");
        let paths = cx
            .get_playlist_track_files(pl_id)
            .expect("failed to retrieve playlist track paths");

        ids.iter()
            .zip(paths.iter())
            .map(|((_, track, album), path)| {
                QueueItemData::new(cx, path.into(), Some(*track), Some(*album))
            })
            .collect()
    } else if let Some(album_id) = track.album_id {
        cx.list_tracks_in_album(album_id)
            .expect("Failed to retrieve tracks")
            .iter()
            .map(|track| {
                QueueItemData::new(cx, track.location.clone(), Some(track.id), track.album_id)
            })
            .collect()
    } else {
        Vec::from([QueueItemData::new(
            cx,
            track.location.clone(),
            Some(track.id),
            track.album_id,
        )])
    };

    replace_queue(queue_items.clone(), cx);

    let playback_interface = cx.global::<GPUIPlaybackInterface>();
    playback_interface.jump_unshuffled(
        queue_items
            .iter()
            .position(|t| t.get_path() == &track.location)
            .unwrap(),
    )
}
