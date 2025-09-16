use std::sync::Arc;

use gpui::{
    div, px, App, AppContext, Context, Entity, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window,
};

use crate::{
    library::{db::LibraryAccess, types::TrackStats},
    ui::{
        components::{
            icons::{icon, DISC},
            sidebar::{sidebar, sidebar_item, sidebar_separator},
        },
        library::sidebar::playlists::PlaylistList,
        theme::Theme,
    },
};

mod playlists;

pub struct Sidebar {
    playlists: Entity<PlaylistList>,
    track_stats: Arc<TrackStats>,
}

impl Sidebar {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            playlists: PlaylistList::new(cx),
            track_stats: cx.get_track_stats().unwrap(),
        })
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let stats_minutes = self.track_stats.total_duration / 60;
        let stats_hours = stats_minutes / 60;

        sidebar()
            .id("sidebar")
            .max_h_full()
            .py(px(12.0))
            .pl(px(12.0))
            .pr(px(11.0))
            .border_r_1()
            .border_color(theme.border_color)
            .child(
                sidebar_item("main-sidebar")
                    .icon(DISC)
                    .child("Albums")
                    .active(),
            )
            .child(sidebar_separator())
            .child(self.playlists.clone())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .mt_auto()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(if self.track_stats.track_count != 1 {
                        format!("{} tracks", self.track_stats.track_count)
                    } else {
                        format!("{} track", self.track_stats.track_count)
                    })
                    .child(format!(
                        "{} hours, {} minutes",
                        stats_hours,
                        stats_minutes % 60
                    )),
            )
    }
}
