use std::collections::VecDeque;

use gpui::*;
use prelude::FluentBuilder;
use tracing::debug;

use crate::{
    library::db::{AlbumMethod, LibraryAccess},
    ui::{
        components::icons::{icon, ARROW_LEFT},
        theme::Theme,
    },
};

use super::ViewSwitchMessage;

pub(super) struct NavigationView {
    view_switcher_model: Entity<VecDeque<ViewSwitchMessage>>,
    current_message: ViewSwitchMessage,
    description: Option<SharedString>,
}

impl NavigationView {
    pub(super) fn new(
        cx: &mut App,
        view_switcher_model: Entity<VecDeque<ViewSwitchMessage>>,
    ) -> Entity<Self> {
        cx.new(|cx| {
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
                        .get_album_by_id(id, AlbumMethod::Thumbnail)
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div().flex().child(
            div()
                .flex()
                .w_full()
                .max_w(px(1000.0))
                .mr_auto()
                .pl(px(10.0))
                .pt(px(10.0))
                .child(
                    div()
                        .flex()
                        .id("back")
                        .size(px(28.0))
                        .flex()
                        .justify_center()
                        .items_center()
                        .rounded_sm()
                        .text_sm()
                        .hover(|this| this.bg(theme.nav_button_hover))
                        .active(|this| this.bg(theme.nav_button_active))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.view_switcher_model.update(cx, |_, cx| {
                                cx.emit(ViewSwitchMessage::Back);
                            })
                        }))
                        .child(icon(ARROW_LEFT).size(px(16.0))),
                ), // .child(
                   //     div()
                   //         .pt(px(5.0))
                   //         .flex()
                   //         .child(div().text_sm().child(match self.current_message {
                   //             ViewSwitchMessage::Albums => "Albums",
                   //             ViewSwitchMessage::Release(_) => "Release",
                   //             ViewSwitchMessage::Back => {
                   //                 panic!("back should not be in VecDeque<ViewSwitchMessage>")
                   //             }
                   //         }))
                   //         .when_some(self.description.clone(), |this, description| {
                   //             this.child(
                   //                 div()
                   //                     .ml(px(8.0))
                   //                     .font_weight(FontWeight::BOLD)
                   //                     .text_sm()
                   //                     .child(description),
                   //             )
                   //         }),
                   // ),
        )
    }
}
