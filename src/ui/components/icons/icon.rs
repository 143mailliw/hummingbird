use gpui::{svg, IntoElement, RenderOnce, SharedString, StyleRefinement, Styled, Svg};

use crate::ui::theme::Theme;

#[derive(IntoElement)]
pub struct Icon {
    svg: Svg,
    icon: SharedString,
}

impl Styled for Icon {
    fn style(&mut self) -> &mut StyleRefinement {
        self.svg.style()
    }
}

impl RenderOnce for Icon {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let theme = cx.global::<Theme>();

        self.svg.path(self.icon).text_color(theme.text)
    }
}

pub fn icon(icon: impl Into<SharedString>) -> Icon {
    Icon {
        svg: svg(),
        icon: icon.into(),
    }
}
