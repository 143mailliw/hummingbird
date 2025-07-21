use gpui::{prelude::FluentBuilder, *};

use crate::ui::{components::icons::icon, theme::Theme};

type ClickEvHandler = dyn Fn(&ClickEvent, &mut Window, &mut App);

// named like this to not conflict with GPUI's MenuItem
#[derive(IntoElement)]
pub enum CMenuItem {
    Item(
        ElementId,
        Option<SharedString>,
        SharedString,
        Box<ClickEvHandler>,
    ),
    Seperator,
    Header(SharedString),
}

impl RenderOnce for CMenuItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        match self {
            CMenuItem::Item(id, icon_path, name, func) => div()
                .id(id)
                .on_click(func)
                .rounded(px(4.0))
                .flex()
                .px(px(9.0))
                .pt(px(5.0))
                .pb(px(5.0))
                .my(px(-1.0))
                .line_height(rems(1.25))
                .min_w_full()
                .bg(theme.menu_item)
                .hover(|this| this.bg(theme.menu_item_hover))
                .active(|this| this.bg(theme.menu_item_active))
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .child(
                    div()
                        .w(px(16.0))
                        .h(px(16.0))
                        .mr(px(8.0))
                        .pt(px(0.5))
                        .my_auto()
                        .flex()
                        .items_center()
                        .justify_center()
                        .when_some(icon_path, |this, icon_path| {
                            this.child(
                                icon(icon_path)
                                    .size(px(16.0))
                                    .text_color(theme.text_secondary),
                            )
                        }),
                )
                .child(div().child(name))
                .into_any_element(),
            CMenuItem::Seperator => div()
                .w_full()
                .h(px(1.0))
                .bg(theme.elevated_border_color)
                .into_any_element(),
            CMenuItem::Header(_) => div().into_any_element(), // TODO: implement this
        }
    }
}

pub fn menu_item(
    id: impl Into<ElementId>,
    icon: Option<impl Into<SharedString>>,
    text: impl Into<SharedString>,
    func: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> CMenuItem {
    CMenuItem::Item(
        id.into(),
        icon.map(|v| v.into()),
        text.into(),
        Box::new(func),
    )
}

#[derive(IntoElement)]
pub struct Menu {
    pub(self) items: Vec<CMenuItem>,
    pub(self) div: Div,
}

impl Menu {
    pub fn item(mut self, item: CMenuItem) -> Self {
        self.items.push(item);

        self
    }
}

impl RenderOnce for Menu {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.div.flex().flex_col().children(self.items)
    }
}

pub fn menu() -> Menu {
    Menu {
        items: vec![],
        div: div().min_w(px(200.0)).px(px(2.0)).py(px(3.0)),
    }
}
