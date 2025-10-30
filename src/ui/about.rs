use gpui::{
    FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, div, img, px,
};

use super::{
    components::modal::{OnExitHandler, modal},
    theme::Theme,
};

const ISSUES_URL: &str = "https://github.com/143mailliw/hummingbird/issues";
const SOURCE_URL: &str = "https://github.com/143mailliw/hummingbird";
const LICENSE_URL: &str = "https://choosealicense.com/licenses/apache-2.0/";

#[derive(IntoElement)]
pub struct AboutDialog {
    on_exit: &'static OnExitHandler,
}

impl RenderOnce for AboutDialog {
    fn render(self, _: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let theme = cx.global::<Theme>();
        let version = env!("CARGO_PKG_VERSION");
        let hash = env!("VERGEN_GIT_SHA");

        modal().on_exit(self.on_exit).child(
            div()
                .p(px(20.0))
                .pb(px(18.0))
                .flex()
                .child(img("!bundled:images/logo.png").w(px(66.0)).mr(px(20.0)))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            div().flex().mr(px(200.0)).child(
                                div()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::BOLD)
                                            .font_family("Lexend")
                                            .text_size(px(36.0))
                                            .line_height(px(36.0))
                                            .ml(px(-2.0))
                                            .child("Hummingbird"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(13.0))
                                            .line_height(px(13.0))
                                            .text_color(theme.text_secondary)
                                            .mt(px(1.0))
                                            .child(format!("{version} ({hash})")),
                                    ),
                            ),
                        )
                        .child(
                            div().mt(px(15.0)).flex().child(
                                div()
                                    .text_sm()
                                    .text_size(px(13.0))
                                    .text_color(theme.text_secondary)
                                    .child(
                                        div()
                                            .flex()
                                            .child(
                                                div()
                                                    .id("about-bug-link")
                                                    .cursor_pointer()
                                                    .text_color(theme.text_link)
                                                    .hover(|this| {
                                                        this.border_b_1()
                                                            .border_color(theme.text_link)
                                                    })
                                                    .on_click(|_, _, cx| {
                                                        cx.open_url(ISSUES_URL);
                                                    })
                                                    .child("Report a bug"),
                                            )
                                            .child(" or ")
                                            .child(
                                                div()
                                                    .id("about-source-link")
                                                    .cursor_pointer()
                                                    .text_color(theme.text_link)
                                                    .hover(|this| {
                                                        this.border_b_1()
                                                            .border_color(theme.text_link)
                                                    })
                                                    .on_click(|_, _, cx| {
                                                        cx.open_url(SOURCE_URL);
                                                    })
                                                    .child("view the source code"),
                                            )
                                            .child(" on GitHub."),
                                    )
                                    .child(div().child(
                                        "Copyright © 2024 - 2025 William Whittaker and \
                                        contributors.",
                                    ))
                                    .child(
                                        div()
                                            .flex()
                                            .child(
                                                "Licensed under the Apache License, version 2.0. ",
                                            )
                                            .child(
                                                div()
                                                    .id("about-rights-link")
                                                    .cursor_pointer()
                                                    .text_color(theme.text_link)
                                                    .hover(|this| {
                                                        this.border_b_1()
                                                            .border_color(theme.text_link)
                                                    })
                                                    .on_click(|_, _, cx| {
                                                        cx.open_url(LICENSE_URL);
                                                    })
                                                    .child("Learn more about your rights."),
                                            ),
                                    ),
                            ),
                        ),
                ),
        )
    }
}

pub fn about_dialog(on_exit: &'static OnExitHandler) -> AboutDialog {
    AboutDialog { on_exit }
}
