use gpui::{
    div, px, rgb, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled,
};

mod playlists;

#[derive(IntoElement)]
pub struct Sidebar {}

impl RenderOnce for Sidebar {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        div()
            .id("sidebar")
            .max_h_full()
            .w(px(225.0))
            .overflow_y_scroll()
            .child("Sidebar")
    }
}
