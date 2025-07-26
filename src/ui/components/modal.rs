use std::rc::Rc;

use gpui::*;
use prelude::FluentBuilder;

use crate::ui::theme::Theme;

pub type OnExitHandler = dyn Fn(&mut Window, &mut App);

#[derive(IntoElement)]
pub struct Modal {
    div: Stateful<Div>,
    on_exit: Option<Rc<OnExitHandler>>,
}

impl Modal {
    fn new() -> Modal {
        Modal {
            div: div().id("modal-fg"),
            on_exit: None,
        }
    }

    pub fn on_exit(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_exit = Some(Rc::new(handler));
        self
    }
}

actions!(modal, [CloseModal]);

pub fn bind_actions(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", CloseModal, None)]);
}

impl ParentElement for Modal {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.div.extend(elements);
    }
}

impl RenderOnce for Modal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let size = window.viewport_size();

        anchored().position(point(px(0.0), px(0.0))).child(deferred(
            div()
                .flex()
                .w(size.width)
                .h(size.height)
                .bg(theme.modal_overlay_bg)
                .id("modal-bg")
                .when_some(self.on_exit, |this, on_exit| {
                    let on_exit_clone = Rc::clone(&on_exit);
                    this.on_any_mouse_down(move |_, window, cx| {
                        on_exit_clone(window, cx);
                    })
                    .on_action(move |_: &CloseModal, window, cx| {
                        on_exit(window, cx);
                    })
                })
                .child(
                    self.div
                        .occlude()
                        .m_auto()
                        .border_color(theme.border_color)
                        .border_1()
                        .bg(theme.background_secondary)
                        .rounded(px(8.0))
                        .flex_col()
                        .on_any_mouse_down(|_, _, cx| {
                            cx.stop_propagation();
                        }),
                ),
        ))
    }
}

pub fn modal() -> Modal {
    Modal::new()
}
