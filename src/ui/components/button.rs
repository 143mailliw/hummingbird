use gpui::*;

use crate::ui::theme::Theme;

use super::styling::AdditionalStyleUtil;

#[derive(Clone, Copy)]
pub enum ButtonSize {
    Regular,
    Large,
}

#[derive(Clone, Copy)]
pub enum ButtonIntent {
    Primary,
    Secondary,
    Warning,
    Danger,
}

#[derive(Clone, Copy)]
pub enum ButtonStyle {
    Regular,
    Minimal,
    MinimalNoRounding,
}

impl ButtonStyle {
    fn base<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        let div = dest.cursor_pointer().flex();

        match self {
            ButtonStyle::Regular => div.shadow_md().rounded(px(4.0)),
            ButtonStyle::Minimal => div.background_opacity(0.0).rounded(px(4.0)),
            ButtonStyle::MinimalNoRounding => div.background_opacity(0.0),
        }
    }

    fn hover<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonStyle::Regular => dest,
            ButtonStyle::Minimal | ButtonStyle::MinimalNoRounding => dest.background_opacity(0.5),
        }
    }

    fn active<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonStyle::Regular => dest,
            ButtonStyle::Minimal | ButtonStyle::MinimalNoRounding => dest.background_opacity(0.5),
        }
    }
}

impl ButtonSize {
    fn base<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonSize::Regular => dest.px(px(10.0)).py(px(3.0)).text_sm().gap(px(8.0)),
            ButtonSize::Large => dest
                .px(px(12.0))
                .pt(px(4.0))
                .pb(px(3.0))
                .text_sm()
                .gap(px(8.0)),
        }
    }

    // i have no idea what this is about but the text size changes when you click on it unless we
    // have this
    fn active<T>(&self, dest: T) -> T
    where
        T: Styled,
    {
        match self {
            ButtonSize::Regular => dest,
            ButtonSize::Large => dest,
        }
    }
}

impl ButtonIntent {
    fn base<T>(&self, dest: T, cx: &mut App) -> T
    where
        T: Styled,
    {
        let theme = cx.global::<Theme>();

        match self {
            ButtonIntent::Primary => dest
                .bg(theme.button_primary)
                .text_color(theme.button_primary_text),
            ButtonIntent::Secondary => dest
                .bg(theme.button_secondary)
                .text_color(theme.button_secondary_text),
            ButtonIntent::Warning => dest
                .bg(theme.button_warning)
                .text_color(theme.button_warning_text),
            ButtonIntent::Danger => dest
                .bg(theme.button_danger)
                .text_color(theme.button_danger_text),
        }
    }
    fn hover<T>(&self, dest: T, cx: &mut App) -> T
    where
        T: Styled,
    {
        let theme = cx.global::<Theme>();

        match self {
            ButtonIntent::Primary => dest.bg(theme.button_primary_hover),
            ButtonIntent::Secondary => dest.bg(theme.button_secondary_hover),
            ButtonIntent::Warning => dest.bg(theme.button_warning_hover),
            ButtonIntent::Danger => dest.bg(theme.button_danger_hover),
        }
    }
    fn active<T>(&self, dest: T, cx: &mut App) -> T
    where
        T: Styled,
    {
        let theme = cx.global::<Theme>();

        match self {
            ButtonIntent::Primary => dest.bg(theme.button_primary_active),
            ButtonIntent::Secondary => dest.bg(theme.button_secondary_active),
            ButtonIntent::Warning => dest.bg(theme.button_warning_active),
            ButtonIntent::Danger => dest.bg(theme.button_danger_active),
        }
    }
}

#[derive(IntoElement)]
pub struct Button {
    pub(self) div: Div,
    pub(self) style: ButtonStyle,
    pub(self) size: ButtonSize,
    pub(self) intent: ButtonIntent,
}

impl Button {
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn intent(mut self, intent: ButtonIntent) -> Self {
        self.intent = intent;
        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn id(self, id: impl Into<ElementId>) -> InteractiveButton {
        InteractiveButton {
            div: self.div.id(id),
            size: self.size,
            style: self.style,
            intent: self.intent,
        }
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        self.div.style()
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.div.extend(elements);
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = self.style;
        let size = self.size;
        let intent = self.intent;

        style.base(size.base(intent.base(self.div.hover(|v| style.hover(intent.hover(v, cx))), cx)))
    }
}

#[derive(IntoElement)]
pub struct InteractiveButton {
    pub(self) div: Stateful<Div>,
    pub(self) style: ButtonStyle,
    pub(self) size: ButtonSize,
    pub(self) intent: ButtonIntent,
}

impl InteractiveButton {
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn intent(mut self, intent: ButtonIntent) -> Self {
        self.intent = intent;
        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn on_click(mut self, fun: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        self.div = self.div.on_click(fun);
        self
    }
}

impl Styled for InteractiveButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.div.style()
    }
}

impl ParentElement for InteractiveButton {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.div.extend(elements);
    }
}

impl RenderOnce for InteractiveButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let style = self.style;
        let size = self.size;
        let intent = self.intent;

        style.base(
            size.base(
                intent.base(
                    self.div
                        .hover(|v| style.hover(intent.hover(v, cx)))
                        .active(|v| style.active(size.active(intent.active(v, cx)))),
                    cx,
                ),
            ),
        )
    }
}

pub fn button() -> Button {
    Button {
        div: div(),
        style: ButtonStyle::Regular,
        size: ButtonSize::Regular,
        intent: ButtonIntent::Secondary,
    }
}
