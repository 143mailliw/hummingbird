use crate::playback::{interface::GPUIPlaybackInterface, thread::PlaybackState};
use gpui::*;
use prelude::FluentBuilder;

use super::{
    components::slider::slider,
    constants::{APP_ROUNDING, FONT_AWESOME},
    global_actions::{Next, PlayPause, Previous},
    models::{Models, PlaybackInfo},
    theme::Theme,
};

pub struct Controls {
    info_section: Entity<InfoSection>,
    scrubber: Entity<Scrubber>,
    secondary_controls: Entity<SecondaryControls>,
}

impl Controls {
    pub fn new(cx: &mut App, show_queue: Entity<bool>) -> Entity<Self> {
        cx.new(|cx| Self {
            info_section: InfoSection::new(cx),
            scrubber: Scrubber::new(cx),
            secondary_controls: SecondaryControls::new(cx, show_queue),
        })
    }
}

impl Render for Controls {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let decorations = window.window_decorations();
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .bg(theme.background_secondary)
            .border_t_1()
            .border_color(theme.border_color)
            .map(|div| match decorations {
                Decorations::Server => div,
                Decorations::Client { tiling } => div
                    .when(!(tiling.bottom || tiling.left), |div| {
                        div.rounded_bl(APP_ROUNDING)
                    })
                    .when(!(tiling.bottom || tiling.right), |div| {
                        div.rounded_br(APP_ROUNDING)
                    }),
            })
            .flex()
            .child(self.info_section.clone())
            .child(self.scrubber.clone())
            .child(self.secondary_controls.clone())
    }
}

pub struct InfoSection {
    track_name: Option<SharedString>,
    artist_name: Option<SharedString>,
    albumart_actual: Option<ImageSource>,
    playback_info: PlaybackInfo,
}

impl InfoSection {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let metadata_model = cx.global::<Models>().metadata.clone();
            let albumart_model = cx.global::<Models>().albumart.clone();
            let playback_info = cx.global::<PlaybackInfo>().clone();

            cx.observe(&playback_info.playback_state, |_, _, cx| {
                cx.notify();
            })
            .detach();

            cx.observe(&metadata_model, |this: &mut Self, m, cx| {
                let metadata = m.read(cx);

                this.track_name = metadata.name.clone().map(SharedString::from);
                this.artist_name = metadata.artist.clone().map(SharedString::from);

                cx.notify();
            })
            .detach();

            cx.observe(&albumart_model, |this: &mut Self, m, cx| {
                let image = m.read(cx).clone();

                this.albumart_actual = image.map(ImageSource::Render);
                cx.notify()
            })
            .detach();

            Self {
                artist_name: None,
                track_name: None,
                albumart_actual: None,
                playback_info,
            }
        })
    }
}

impl Render for InfoSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = self.playback_info.playback_state.read(cx);

        div()
            .id("info-section")
            .flex()
            .w(px(275.0))
            .min_w(px(275.0))
            .max_w(px(275.0))
            .overflow_x_hidden()
            .flex_shrink_0()
            .child(
                div()
                    .mx(px(12.0))
                    .mt(px(12.0))
                    .mb(px(6.0))
                    .gap(px(10.0))
                    .flex()
                    .overflow_x_hidden()
                    .child(
                        div()
                            .id("album-art")
                            .rounded(px(4.0))
                            .bg(theme.album_art_background)
                            .shadow_sm()
                            .w(px(36.0))
                            .h(px(36.0))
                            .mb(px(6.0))
                            .when(self.albumart_actual.is_some(), |div| {
                                div.child(
                                    img(self.albumart_actual.clone().unwrap())
                                        .w(px(36.0))
                                        .h(px(36.0))
                                        .rounded(px(4.0)),
                                )
                            }),
                    )
                    .when(*state == PlaybackState::Stopped, |e| {
                        e.child(
                            div()
                                .line_height(rems(1.0))
                                .font_weight(FontWeight::EXTRA_BOLD)
                                .text_size(px(15.0))
                                .flex()
                                .h_full()
                                .items_center()
                                .pb(px(6.0))
                                .child("Muzak"),
                        )
                    })
                    .when(*state != PlaybackState::Stopped, |e| {
                        e.child(
                            div()
                                .flex()
                                .flex_col()
                                .line_height(rems(1.0))
                                .text_size(px(15.0))
                                .gap_1()
                                .overflow_x_hidden()
                                .child(
                                    div()
                                        .overflow_x_hidden()
                                        .font_weight(FontWeight::EXTRA_BOLD)
                                        .text_ellipsis()
                                        .child(
                                            self.track_name
                                                .clone()
                                                .unwrap_or("Unknown Track".into()),
                                        ),
                                )
                                .child(
                                    div()
                                        .overflow_x_hidden()
                                        .pb(px(6.0))
                                        .text_ellipsis()
                                        .overflow_x_hidden()
                                        .child(
                                            self.artist_name
                                                .clone()
                                                .unwrap_or("Unknown Artist".into()),
                                        ),
                                ),
                        )
                    }),
            )
    }
}

pub struct PlaybackSection {
    info: PlaybackInfo,
}

impl PlaybackSection {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let info = cx.global::<PlaybackInfo>().clone();
            let state = info.playback_state.clone();
            let shuffling = info.shuffling.clone();

            cx.observe(&state, |_, _, cx| {
                cx.notify();
            })
            .detach();

            cx.observe(&shuffling, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self { info }
        })
    }
}

impl Render for PlaybackSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.info.playback_state.read(cx);
        let shuffling = self.info.shuffling.read(cx);
        let repeating = self.info.repeating.read(cx);
        let theme = cx.global::<Theme>();

        div()
            .mr(auto())
            .ml(auto())
            .mt(px(5.0))
            .flex()
            .w_full()
            .absolute()
            .child(
                div()
                    .rounded(px(3.0))
                    .w(px(28.0))
                    .h(px(25.0))
                    .mt(px(2.0))
                    .mr(px(6.0))
                    .ml_auto()
                    .border_color(theme.playback_button_border)
                    .font_family(FONT_AWESOME)
                    .text_size(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(|style| style.bg(theme.playback_button_hover).cursor_pointer())
                    .id("header-shuffle-button")
                    .active(|style| style.bg(theme.playback_button_active))
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        cx.stop_propagation();
                        window.prevent_default();
                    })
                    .on_click(|_, _, cx| {
                        cx.global::<GPUIPlaybackInterface>().toggle_shuffle();
                    })
                    .child("")
                    .when(*shuffling, |this| {
                        this.text_color(theme.playback_button_toggled)
                    }),
            )
            .child(
                div()
                    .rounded(px(4.0))
                    .border_color(theme.playback_button_border)
                    .border_1()
                    .flex()
                    .child(
                        div()
                            .w(px(30.0))
                            .h(px(28.0))
                            .rounded_l(px(3.0))
                            .bg(theme.playback_button)
                            .font_family(FONT_AWESOME)
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|style| style.bg(theme.playback_button_hover).cursor_pointer())
                            .id("header-prev-button")
                            .active(|style| style.bg(theme.playback_button_active))
                            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                cx.stop_propagation();
                                window.prevent_default();
                            })
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(Previous), cx);
                            })
                            // icon: `backward-step`
                            // https://fontawesome.com/icons/backward-step?f=classic&s=solid
                            .child("\u{f048}"),
                    )
                    .child(
                        div()
                            .w(px(32.0))
                            .h(px(28.0))
                            .bg(theme.playback_button)
                            .border_l(px(1.0))
                            .border_r(px(1.0))
                            .border_color(theme.playback_button_border)
                            .font_family(FONT_AWESOME)
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|style| style.bg(theme.playback_button_hover).cursor_pointer())
                            .id("header-play-button")
                            .active(|style| style.bg(theme.playback_button_active))
                            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                cx.stop_propagation();
                                window.prevent_default();
                            })
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(PlayPause), cx);
                            })
                            .when(*state == PlaybackState::Playing, |div| div.child(""))
                            .when(*state != PlaybackState::Playing, |div| div.child("")),
                    )
                    .child(
                        div()
                            .w(px(30.0))
                            .h(px(28.0))
                            .rounded_r(px(3.0))
                            .bg(theme.playback_button)
                            .font_family(FONT_AWESOME)
                            .flex()
                            .items_center()
                            .justify_center()
                            .hover(|style| style.bg(theme.playback_button_hover).cursor_pointer())
                            .id("header-next-button")
                            .active(|style| style.bg(theme.playback_button_active))
                            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                cx.stop_propagation();
                                window.prevent_default();
                            })
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(Next), cx);
                            })
                            // icon: `forward-step`
                            // https://fontawesome.com/icons/forward-step?f=classic&s=solid
                            .child("\u{f051}"),
                    ),
            )
            .child(
                div()
                    .rounded(px(3.0))
                    .w(px(28.0))
                    .h(px(25.0))
                    .mt(px(2.0))
                    .ml(px(6.0))
                    .mr_auto()
                    .border_color(theme.playback_button_border)
                    .font_family(FONT_AWESOME)
                    .text_size(px(12.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .hover(|style| style.bg(theme.playback_button_hover).cursor_pointer())
                    .id("header-repeat-button")
                    .active(|style| style.bg(theme.playback_button_active))
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        cx.stop_propagation();
                        window.prevent_default();
                    })
                    .on_click(|_, _, cx| {
                        cx.global::<GPUIPlaybackInterface>().toggle_repeat();
                    })
                    .child("")
                    .when(*repeating, |this| {
                        this.text_color(theme.playback_button_toggled)
                    }),
            )
    }
}

pub struct Scrubber {
    position: Entity<u64>,
    duration: Entity<u64>,
    playback_section: Entity<PlaybackSection>,
}

impl Scrubber {
    fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let position_model = cx.global::<PlaybackInfo>().position.clone();
            let duration_model = cx.global::<PlaybackInfo>().duration.clone();

            cx.observe(&position_model, |_, _, cx| {
                cx.notify();
            })
            .detach();

            cx.observe(&duration_model, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self {
                position: position_model,
                duration: duration_model,
                playback_section: PlaybackSection::new(cx),
            }
        })
    }
}

impl Render for Scrubber {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let position = *self.position.read(cx);
        let duration = *self.duration.read(cx);
        let remaining = duration - position;
        div()
            .pl(px(13.0))
            .pr(px(13.0))
            .border_x(px(1.0))
            .border_color(theme.border_color)
            .flex_grow()
            .flex()
            .flex_col()
            .line_height(rems(1.0))
            .text_size(px(15.0))
            .font_family("Roboto Mono")
            .font_weight(FontWeight::BOLD)
            .child(
                div()
                    .w_full()
                    .flex()
                    .relative()
                    .items_end()
                    .mt(px(6.0))
                    .mb(px(6.0))
                    .child(
                        div()
                            .pr(px(6.0))
                            .border_r(px(2.0))
                            .border_color(rgb(0x4b5563))
                            .child(format!("{:02}:{:02}", position / 60, position % 60)),
                    )
                    .child(div().ml(px(6.0)).text_color(rgb(0xcbd5e1)).child(format!(
                        "{:02}:{:02}",
                        duration / 60,
                        duration % 60
                    )))
                    .child(deferred(self.playback_section.clone()))
                    .child(div().h(px(30.0)))
                    .child(div().ml(auto()).child(format!(
                        "-{:02}:{:02}",
                        remaining / 60,
                        remaining % 60
                    ))),
            )
            .child(
                slider()
                    .w_full()
                    .h(px(6.0))
                    .rounded(px(3.0))
                    .id("scrubber-back")
                    .value(position as f32 / duration as f32)
                    .on_change(move |v, _, cx| {
                        let info = cx.global::<PlaybackInfo>().clone();

                        if duration > 0 && *info.playback_state.read(cx) != PlaybackState::Stopped {
                            cx.global::<GPUIPlaybackInterface>()
                                .seek(v as f64 * duration as f64);
                        }
                    }),
            )
    }
}

pub struct SecondaryControls {
    info: PlaybackInfo,
    show_queue: Entity<bool>,
}

impl SecondaryControls {
    pub fn new(cx: &mut App, show_queue: Entity<bool>) -> Entity<Self> {
        cx.new(|cx| {
            let info = cx.global::<PlaybackInfo>().clone();
            let volume = info.volume.clone();

            cx.observe(&volume, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self { info, show_queue }
        })
    }
}

impl Render for SecondaryControls {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let volume = *self.info.volume.read(cx);
        let prev_volume = *self.info.prev_volume.read(cx);
        let show_queue = self.show_queue.clone();

        div().px(px(18.0)).flex().child(
            div()
                .flex()
                .my_auto()
                .pb(px(2.0))
                .gap(px(8.0))
                .child(
                    div()
                        .rounded(px(3.0))
                        .w(px(28.0))
                        .h(px(25.0))
                        .mt(px(2.0))
                        .font_family(FONT_AWESOME)
                        .text_size(px(12.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_color(theme.playback_button_border)
                        .id("volume-button")
                        .bg(theme.playback_button)
                        .hover(|this| this.bg(theme.playback_button_hover))
                        .active(|this| this.bg(theme.playback_button_active))
                        .when(volume <= 0.0, |div| {
                            // icon: `volume-xmark`
                            // https://fontawesome.com/icons/volume-xmark?f=classic&s=solid
                            div.child("\u{f6a9}").on_click(move |_, _, cx| {
                                cx.global::<GPUIPlaybackInterface>().set_volume(prev_volume);
                            })
                        })
                        .when(volume > 0.0, |div| {
                            // icon: `volume-high`
                            // https://fontawesome.com/icons/volume-high?f=classic&s=solid
                            div.child("").on_click(move |_, _, cx| {
                                cx.global::<GPUIPlaybackInterface>().set_volume(0 as f64);
                            })
                        }),
                )
                .child(
                    slider()
                        .w(px(80.0))
                        .h(px(6.0))
                        .mt(px(11.0))
                        .rounded(px(3.0))
                        .id("volume")
                        .value((volume) as f32)
                        .on_change(move |v, _, cx| {
                            cx.global::<GPUIPlaybackInterface>().set_volume(v as f64);
                        }),
                )
                .child(
                    div()
                        .rounded(px(3.0))
                        .w(px(28.0))
                        .h(px(25.0))
                        .mt(px(2.0))
                        .font_family(FONT_AWESOME)
                        .text_size(px(12.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_color(theme.playback_button_border)
                        .id("queue-button")
                        .bg(theme.playback_button)
                        .hover(|this| this.bg(theme.playback_button_hover))
                        .active(|this| this.bg(theme.playback_button_active))
                        .child("")
                        .on_click(move |_, _, cx| {
                            show_queue.update(cx, |m, cx| {
                                *m = !*m;
                                cx.notify();
                            })
                        }),
                ),
        )
    }
}

pub struct EmptyView;

impl Render for EmptyView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
