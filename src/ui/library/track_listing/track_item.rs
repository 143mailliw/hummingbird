use std::sync::Arc;

use gpui::prelude::{FluentBuilder, *};
use gpui::{div, px, App, Entity, FontWeight, IntoElement, RenderOnce, Window};

use crate::ui::components::icons::{PLAY, PLUS};
use crate::{
    library::{db::LibraryAccess, types::Track},
    playback::{
        interface::{replace_queue, GPUIPlaybackInterface},
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

pub struct TrackItem {
    pub track: Arc<Track>,
    pub is_start: bool,
    pub artist_name_visibility: ArtistNameVisibility,
}

impl TrackItem {
    pub fn new(
        cx: &mut App,
        track: Arc<Track>,
        is_start: bool,
        anv: ArtistNameVisibility,
    ) -> Entity<TrackItem> {
        cx.new(|cx| TrackItem {
            track,
            is_start,
            artist_name_visibility: anv,
        })
    }
}

impl Render for TrackItem {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let current_track = cx.global::<PlaybackInfo>().current_track.read(cx).clone();
        let track = self.track.clone();

        let track_location = track.location.clone();
        let track_location_2 = track.location.clone();
        let track_id = track.id;
        let album_id = track.album_id;

        let show_artist_name = self.artist_name_visibility != ArtistNameVisibility::Never
            && self.artist_name_visibility
                != ArtistNameVisibility::OnlyIfDifferent(track.artist_names.clone());

        context(("context", track.id as usize))
            .with(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .id(track.id as usize)
                    .on_click({
                        let track = track.clone();
                        move |_, _, cx| play_from_track(cx, &track)
                    })
                    .when(self.is_start, |this| {
                        this.child(
                            div()
                                .text_color(theme.text_secondary)
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .px(px(24.0))
                                .border_b_1()
                                .w_full()
                                .border_color(theme.border_color)
                                .mt(px(24.0))
                                .pb(px(6.0))
                                .when_some(track.disc_number, |this, num| {
                                    this.child(format!("DISC {num}"))
                                }),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .border_b_1()
                            .id(("track", track.id as u64))
                            .w_full()
                            .border_color(theme.border_color)
                            .cursor_pointer()
                            .px(px(24.0))
                            .py(px(6.0))
                            .hover(|this| this.bg(theme.nav_button_hover))
                            .active(|this| this.bg(theme.nav_button_active))
                            .when_some(current_track, |this, curr_track| {
                                this.bg(if curr_track == track.location {
                                    theme.queue_item_current
                                } else {
                                    theme.background_primary
                                })
                            })
                            .max_w_full()
                            .child(
                                div()
                                    .w(px(62.0))
                                    .flex_shrink_0()
                                    .child(format!("{}", track.track_number.unwrap_or_default())),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .overflow_x_hidden()
                                    .text_ellipsis()
                                    .child(track.title.clone()),
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
                                    .ml_auto()
                                    .when(show_artist_name, |this| {
                                        this.when_some(track.artist_names.clone(), |this, v| {
                                            this.child(v.0)
                                        })
                                    }),
                            )
                            .child(div().ml(px(12.0)).flex_shrink_0().child(format!(
                                "{}:{:02}",
                                track.duration / 60,
                                track.duration % 60
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
                            move |_, _, cx| play_from_track(cx, &track),
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

pub fn play_from_track(cx: &mut App, track: &Track) {
    let queue_items = if let Some(album_id) = track.album_id {
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
