use std::sync::Arc;

use gpui::{
    div, px, App, AppContext, Context, Entity, FontWeight, ParentElement, Render, Styled, Window,
};

use crate::{
    library::{
        db::LibraryAccess,
        types::{PlaylistType, PlaylistWithCount},
    },
    ui::{
        components::{
            icons::{PLAYLIST, STAR},
            sidebar::sidebar_item,
        },
        models::{Models, PlaylistEvent},
        theme::Theme,
    },
};

pub struct PlaylistList {
    playlists: Arc<Vec<PlaylistWithCount>>,
}

impl PlaylistList {
    pub fn new(cx: &mut App) -> Entity<Self> {
        let playlists = cx.get_all_playlists().expect("could not geet playlists");

        cx.new(|cx| {
            let playlist_tracker = cx.global::<Models>().playlist_tracker.clone();

            cx.subscribe(
                &playlist_tracker,
                |this: &mut Self, _, _: &PlaylistEvent, cx| {
                    this.playlists = cx.get_all_playlists().unwrap();

                    cx.notify();
                },
            )
            .detach();

            Self {
                playlists: playlists.clone(),
            }
        })
    }
}

impl Render for PlaylistList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.global::<Theme>();
        let mut main = div();

        for playlist in &*self.playlists {
            main = main.child(
                sidebar_item(("main-sidebar-pl", playlist.id as u64))
                    .icon(if playlist.playlist_type == PlaylistType::System {
                        STAR
                    } else {
                        PLAYLIST
                    })
                    .child(playlist.name.clone())
                    .child(
                        div()
                            .font_weight(FontWeight::NORMAL)
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .mt(px(2.0))
                            .child(if playlist.track_count == 1 {
                                format!("{} song", playlist.track_count)
                            } else {
                                format!("{} songs", playlist.track_count)
                            }),
                    ),
            )
        }

        main
    }
}
