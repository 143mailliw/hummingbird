use std::{collections::VecDeque, sync::Arc};

use gpui::{
    App, AppContext, Context, Entity, FontWeight, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
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
        library::ViewSwitchMessage,
        models::{Models, PlaylistEvent},
        theme::Theme,
    },
};

pub struct PlaylistList {
    playlists: Arc<Vec<PlaylistWithCount>>,
    nav_model: Entity<VecDeque<ViewSwitchMessage>>,
}

impl PlaylistList {
    pub fn new(cx: &mut App, nav_model: Entity<VecDeque<ViewSwitchMessage>>) -> Entity<Self> {
        let playlists = cx.get_all_playlists().expect("could not get playlists");

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

            cx.observe(&nav_model, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self {
                playlists: playlists.clone(),
                nav_model,
            }
        })
    }
}

impl Render for PlaylistList {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.global::<Theme>();
        let mut main = div();
        let current_view = self.nav_model.read(cx);

        for playlist in &*self.playlists {
            let pl_id = playlist.id;

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
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.nav_model.update(cx, move |_, cx| {
                            cx.emit(ViewSwitchMessage::Playlist(pl_id));
                        });
                    }))
                    .when(
                        current_view.iter().last()
                            == Some(&ViewSwitchMessage::Playlist(playlist.id)),
                        |this| this.active(),
                    ),
            )
        }

        main
    }
}
