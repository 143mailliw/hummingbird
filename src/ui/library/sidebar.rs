use gpui::{
    div, px, App, AppContext, Context, Entity, FontWeight, InteractiveElement, IntoElement,
    ParentElement, Render, StatefulInteractiveElement, Styled, Window,
};

use crate::ui::{
    components::{
        icons::{icon, DISC},
        sidebar::{sidebar, sidebar_item, sidebar_separator},
    },
    library::sidebar::playlists::PlaylistList,
    theme::Theme,
};

mod playlists;

pub struct Sidebar {
    playlists: Entity<PlaylistList>,
}

impl Sidebar {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self {
            playlists: PlaylistList::new(cx),
        })
    }
}

impl Render for Sidebar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        sidebar()
            .id("sidebar")
            .max_h_full()
            .pt(px(12.0))
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
    }
}
