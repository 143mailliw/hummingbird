use gpui::*;
use tracing::error;

use crate::{
    services::mmb::lastfm::{LASTFM_API_KEY, LASTFM_API_SECRET, client::LastFMClient},
    ui::{
        components::icons::{LAST_FM, icon},
        models::{LastFMState, Models},
        theme::Theme,
    },
};

pub struct LastFM {
    state: Entity<LastFMState>,
    name: Option<SharedString>,
}

impl LastFM {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let models = cx.global::<Models>();
            let state = models.lastfm.clone();

            cx.observe(&state, |this: &mut LastFM, m, cx| {
                this.name = match m.read(cx) {
                    LastFMState::Connected(session) => Some(session.name.clone().into()),
                    _ => None,
                }
            })
            .detach();

            LastFM {
                name: match state.read(cx) {
                    LastFMState::Connected(session) => Some(session.name.clone().into()),
                    _ => None,
                },
                state,
            }
        })
    }
}

impl Render for LastFM {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let state = self.state.clone();

        div()
            .flex()
            .text_sm()
            .px(px(12.0))
            .pb(px(8.0))
            .pt(px(7.0))
            .cursor_pointer()
            .text_color(theme.text_secondary)
            .bg(theme.window_button)
            .id("lastfm-button")
            .hover(|this| this.bg(theme.window_button_hover))
            .active(|this| this.bg(theme.window_button_active))
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                window.prevent_default();
                cx.stop_propagation();
            })
            .child(
                div()
                    .mr(px(8.0))
                    .pt(px(5.5))
                    .text_size(px(11.0))
                    .h_full()
                    .child(
                        icon(LAST_FM)
                            .size(px(14.0))
                            .text_color(theme.text_secondary),
                    ),
            )
            .child(
                div().child(match self.state.read(cx) {
                    LastFMState::Disconnected => "Sign in".into_any_element(),
                    LastFMState::AwaitingFinalization(_) => {
                        "Click to confirm sign in".into_any_element()
                    }
                    LastFMState::Connected(_) => self
                        .name
                        .clone()
                        .unwrap_or(SharedString::new_static("Connected"))
                        .into_any_element(),
                }),
            )
            .on_click(move |_, _, cx| {
                let state = state.clone();
                let read = state.read(cx).clone();

                match read {
                    LastFMState::Disconnected => get_token(cx, state),
                    LastFMState::AwaitingFinalization(token) => confirm(cx, state, token),
                    LastFMState::Connected(_) => (),
                }
            })
    }
}

fn get_token(cx: &mut App, state: Entity<LastFMState>) {
    cx.spawn(async move |cx| {
        let mut client = LastFMClient::new(
            LASTFM_API_KEY.unwrap().to_string(),
            LASTFM_API_SECRET.unwrap(),
        );

        if let Ok(token) = client.get_token().await {
            let path = format!(
                "http://last.fm/api/auth/?api_key={}&token={}",
                LASTFM_API_KEY.unwrap(),
                token
            );
            if open::that(&path).is_err() {
                error!(
                    "Failed to open web browser to {}; you'll need to navigate to it manually.",
                    path
                );
            }
            state
                .update(cx, move |m, cx| {
                    *m = LastFMState::AwaitingFinalization(token);
                    cx.notify();
                })
                .expect("failed to update lastfm state");
        } else {
            error!("error getting token");
        }
    })
    .detach();
}

fn confirm(cx: &mut App, state: Entity<LastFMState>, token: String) {
    cx.spawn(async move |cx| {
        let mut client = LastFMClient::new(
            LASTFM_API_KEY.unwrap().to_string(),
            LASTFM_API_SECRET.unwrap(),
        );

        if let Ok(session) = client.get_session(token).await {
            state
                .update(cx, move |_, cx| {
                    cx.emit(session);
                })
                .expect("failed to emit session event");
        } else {
            error!("error getting session")
        }
    })
    .detach();
}
