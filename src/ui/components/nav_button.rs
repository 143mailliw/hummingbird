use gpui::{
    Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled, div, px,
};

use crate::ui::{components::icons::icon, theme::Theme};

#[derive(IntoElement)]
pub struct NavButton {
    div: Stateful<Div>,
    icon: &'static str,
}

impl StatefulInteractiveElement for NavButton {}

impl InteractiveElement for NavButton {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.div.interactivity()
    }
}

impl Styled for NavButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.div.style()
    }
}

impl RenderOnce for NavButton {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let theme = cx.global::<Theme>();

        self.div
            .flex()
            .size(px(28.0))
            .flex()
            .justify_center()
            .items_center()
            .rounded_sm()
            .text_sm()
            .hover(|this| this.bg(theme.nav_button_hover))
            .active(|this| this.bg(theme.nav_button_active))
            .cursor_pointer()
            .child(icon(self.icon).size(px(16.0)))
    }
}

pub fn nav_button(id: impl Into<ElementId>, icon: &'static str) -> NavButton {
    NavButton {
        div: div().id(id),
        icon,
    }
}
