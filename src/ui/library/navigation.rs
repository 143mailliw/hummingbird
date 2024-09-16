use std::collections::VecDeque;

use gpui::*;
use prelude::FluentBuilder;
use tracing::debug;

use crate::library::db::LibraryAccess;

use super::ViewSwitchMessage;

pub(super) struct NavigationView {
    view_switcher_model: Model<VecDeque<ViewSwitchMessage>>,
    current_message: ViewSwitchMessage,
    description: Option<SharedString>,
}

impl NavigationView {
    pub(super) fn new<V: 'static>(
        cx: &mut ViewContext<V>,
        view_switcher_model: Model<VecDeque<ViewSwitchMessage>>,
    ) -> View<Self> {
        cx.new_view(|cx| {
            let current_message = *view_switcher_model
                .read(cx)
                .back()
                .expect("view_switcher_model should always have one element");

            cx.observe(&view_switcher_model, |this: &mut NavigationView, m, cx| {
                debug!("{:#?}", m.read(cx));

                this.current_message = *m
                    .read(cx)
                    .back()
                    .expect("view_switcher_model should always have one element");

                this.description = match this.current_message {
                    ViewSwitchMessage::Release(id) => cx
                        .get_album_by_id(id)
                        .ok()
                        .map(|v| SharedString::from(v.title.clone())),
                    _ => None,
                }
            })
            .detach();

            Self {
                view_switcher_model,
                current_message,
                description: None,
            }
        })
    }
}

impl Render for NavigationView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div().flex().border_b_1().border_color(rgb(0x1e293b)).child(
            div()
                .flex()
                .w_full()
                .max_w(px(1000.0))
                .mx_auto()
                .px(px(8.0))
                .child(
                    div()
                        .flex()
                        .id("back")
                        .pr(px(19.0))
                        .pl(px(16.0))
                        .py(px(8.0))
                        .mr(px(16.0))
                        .text_sm()
                        .border_r_1()
                        .border_color(rgb(0x1e293b))
                        .hover(|this| this.bg(rgb(0x1e293b)))
                        .active(|this| this.bg(rgb(0x111827)))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, cx| {
                            this.view_switcher_model.update(cx, |_, cx| {
                                cx.emit(ViewSwitchMessage::Back);
                            })
                        }))
                        .child(""),
                )
                .child(
                    div()
                        .pt(px(7.0))
                        .flex()
                        .child(div().text_sm().child(match self.current_message {
                            ViewSwitchMessage::Albums => "Albums",
                            ViewSwitchMessage::Release(_) => "Release",
                            ViewSwitchMessage::Back => {
                                panic!("back should not be in VecDeque<ViewSwitchMessage>")
                            }
                        }))
                        .when_some(self.description.clone(), |this, description| {
                            this.child(
                                div()
                                    .ml(px(12.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_sm()
                                    .child(description),
                            )
                        }),
                ),
        )
    }
}
