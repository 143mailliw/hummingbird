mod lastfm;

use gpui::*;
use prelude::FluentBuilder;

use crate::{
    library::scan::ScanEvent,
    services::mmb::lastfm::{LASTFM_API_KEY, LASTFM_API_SECRET},
    ui::components::icons::{CROSS, FOLDER_CHECK, FOLDER_SEARCH, MAXIMIZE, MINUS, icon},
};

use super::{constants::APP_ROUNDING, models::Models, theme::Theme};

pub struct Header {
    scan_status: Entity<ScanStatus>,
    lastfm: Option<Entity<lastfm::LastFM>>,
}

impl Header {
    pub fn new(cx: &mut App) -> Entity<Self> {
        let lastfm = if LASTFM_API_SECRET.is_some() && LASTFM_API_KEY.is_some() {
            Some(lastfm::LastFM::new(cx))
        } else {
            None
        };

        cx.new(|cx| Self {
            scan_status: ScanStatus::new(cx),
            lastfm,
        })
    }
}

impl Render for Header {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let decorations = window.window_decorations();
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .w_full()
            .text_sm()
            .min_h(px(37.0))
            .max_h(px(37.0))
            .bg(theme.background_secondary)
            .text_sm()
            .border_b_1()
            .id("titlebar")
            .border_color(theme.border_color)
            .window_control_area(WindowControlArea::Drag)
            .when(cfg!(not(target_os = "windows")), |this| {
                this.on_mouse_down(MouseButton::Left, move |ev, window, _| {
                    if ev.click_count != 2 {
                        window.start_window_move();
                    }
                })
                .on_click(|ev, window, _| {
                    if ev.click_count() == 2 {
                        window.zoom_window();
                    }
                })
            })
            .map(|div| match decorations {
                Decorations::Server => div,
                Decorations::Client { tiling } => div
                    .when(!(tiling.top || tiling.left), |div| {
                        div.rounded_tl(APP_ROUNDING)
                    })
                    .when(!(tiling.top || tiling.right), |div| {
                        div.rounded_tr(APP_ROUNDING)
                    }),
            })
            .when(cfg!(target_os = "macos"), |this| {
                this.child(div().w(px(72.0)))
            })
            .child(
                div()
                    .pl(px(12.0))
                    .pb(px(8.0))
                    .pt(px(7.0))
                    .flex()
                    .when(cfg!(not(target_os = "macos")), |this| {
                        this.child(
                            div()
                                .id("hummingbird-name")
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                })
                                .on_click(|_, _, cx| {
                                    let show_about = cx.global::<Models>().show_about.clone();
                                    show_about.write(cx, true);
                                })
                                .child("Hummingbird")
                                .mr(px(8.0)),
                        )
                    })
                    .child(self.scan_status.clone()),
            )
            .child(div().ml_auto())
            .when_some(self.lastfm.clone(), |this, lastfm| this.child(lastfm))
            .when(cfg!(not(target_os = "macos")), |this| {
                this.child(
                    div()
                        .flex()
                        .child(WindowButton::Minimize)
                        .child(WindowButton::Maximize)
                        .child(WindowButton::Close),
                )
            })
    }
}

pub struct ScanStatus {
    scan_model: Entity<ScanEvent>,
}

impl ScanStatus {
    pub fn new(cx: &mut App) -> Entity<Self> {
        let scan_model = cx.global::<Models>().scan_state.clone();

        cx.new(|cx| {
            cx.observe(&scan_model, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self { scan_model }
        })
    }
}

impl Render for ScanStatus {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let status = self.scan_model.read(cx);

        div()
            .flex()
            .text_sm()
            .child(
                div().mr(px(8.0)).pt(px(4.5)).h_full().child(
                    icon(match status {
                        ScanEvent::ScanCompleteIdle | ScanEvent::ScanCompleteWatching => {
                            FOLDER_CHECK
                        }
                        _ => FOLDER_SEARCH,
                    })
                    .size(px(14.0)),
                ),
            )
            .text_color(theme.text_secondary)
            .child(match status {
                ScanEvent::ScanCompleteIdle => "".to_string(),
                ScanEvent::ScanProgress { current, total } => {
                    format!(
                        "Scanning ({}%)",
                        (*current as f64 / *total as f64 * 100.0).round()
                    )
                }
                ScanEvent::DiscoverProgress(progress) => {
                    format!("Discovering files ({progress})")
                }
                ScanEvent::Cleaning => "".to_string(),
                ScanEvent::ScanCompleteWatching => "Watching for updates".to_string(),
            })
    }
}

#[derive(PartialEq, Clone, Copy, IntoElement)]
pub enum WindowButton {
    Close,
    Minimize,
    Maximize,
}

impl RenderOnce for WindowButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let (bg, hover, active) = if self == WindowButton::Close {
            (
                theme.close_button,
                theme.close_button_hover,
                theme.close_button_active,
            )
        } else {
            (
                theme.window_button,
                theme.window_button_hover,
                theme.window_button_active,
            )
        };

        div()
            .flex()
            .w(px(36.0))
            .h(px(37.0))
            .pb(px(1.0))
            .items_center()
            .justify_center()
            .cursor_pointer()
            .id(match self {
                WindowButton::Close => "close",
                WindowButton::Minimize => "minimize",
                WindowButton::Maximize => "maximize",
            })
            .bg(bg)
            .hover(|this| this.bg(hover))
            .active(|this| this.bg(active))
            .window_control_area(match self {
                WindowButton::Close => WindowControlArea::Close,
                WindowButton::Minimize => WindowControlArea::Min,
                WindowButton::Maximize => WindowControlArea::Max,
            })
            .text_size(px(11.0))
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                cx.stop_propagation();
                window.prevent_default();
            })
            .child(
                icon(match self {
                    WindowButton::Close => CROSS,
                    WindowButton::Minimize => MINUS,
                    WindowButton::Maximize => MAXIMIZE,
                })
                .size(px(14.0)),
            )
            .when(self == WindowButton::Close, |this| this.rounded_tr(px(4.0)))
            .on_click(move |_, window, cx| match self {
                WindowButton::Close => cx.quit(),
                WindowButton::Minimize => window.minimize_window(),
                WindowButton::Maximize => window.zoom_window(),
            })
    }
}
