use std::collections::VecDeque;

use album_view::AlbumView;
use gpui::*;
use navigation::NavigationView;
use release_view::ReleaseView;
use tracing::debug;

use crate::ui::library::{playlist_view::PlaylistView, sidebar::Sidebar};

use super::models::Models;

mod album_view;
mod navigation;
mod playlist_view;
mod release_view;
mod sidebar;
mod track_listing;

#[derive(Clone)]
enum LibraryView {
    Album(Entity<AlbumView>),
    Release(Entity<ReleaseView>),
    Playlist(Entity<PlaylistView>),
}

pub struct Library {
    view: LibraryView,
    navigation_view: Entity<NavigationView>,
    sidebar: Entity<Sidebar>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewSwitchMessage {
    Albums,
    Release(i64),
    Playlist(i64),
    Back,
}

impl EventEmitter<ViewSwitchMessage> for VecDeque<ViewSwitchMessage> {}

fn make_view(
    message: &ViewSwitchMessage,
    cx: &mut App,
    model: Entity<VecDeque<ViewSwitchMessage>>,
) -> LibraryView {
    match message {
        ViewSwitchMessage::Albums => LibraryView::Album(AlbumView::new(cx, model.clone())),
        ViewSwitchMessage::Release(id) => LibraryView::Release(ReleaseView::new(cx, *id)),
        ViewSwitchMessage::Playlist(id) => LibraryView::Playlist(PlaylistView::new(cx, *id)),
        ViewSwitchMessage::Back => panic!("improper use of make_view (cannot make Back)"),
    }
}

impl Library {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let switcher_model = cx.global::<Models>().switcher_model.clone();
            let view = LibraryView::Album(AlbumView::new(cx, switcher_model.clone()));

            cx.subscribe(
                &switcher_model,
                move |this: &mut Library, m, message, cx| {
                    this.view = match message {
                        ViewSwitchMessage::Back => {
                            let last = m.update(cx, |v: &mut VecDeque<ViewSwitchMessage>, cx| {
                                if v.len() > 1 {
                                    v.pop_back();
                                    cx.notify();

                                    v.back().cloned()
                                } else {
                                    None
                                }
                            });

                            if let Some(message) = last {
                                debug!("{:?}", message);
                                make_view(&message, cx, m)
                            } else {
                                this.view.clone()
                            }
                        }
                        _ => {
                            m.update(cx, |v, cx| {
                                if v.len() > 99 {
                                    v.pop_front();
                                }
                                v.push_back(*message);

                                cx.notify();
                            });

                            make_view(message, cx, m)
                        }
                    };

                    cx.notify();
                },
            )
            .detach();

            Library {
                navigation_view: NavigationView::new(cx, switcher_model.clone()),
                sidebar: Sidebar::new(cx, switcher_model.clone()),
                view,
            }
        })
    }
}

impl Render for Library {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h_full()
            .flex()
            .flex_shrink()
            .max_w_full()
            .max_h_full()
            .overflow_hidden()
            .child(
                div()
                    .mr_auto()
                    .flex()
                    .flex_shrink_0()
                    .child(self.sidebar.clone()),
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(1000.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .flex_shrink()
                    .mr_auto()
                    .overflow_hidden()
                    .child(self.navigation_view.clone())
                    .child(match &self.view {
                        LibraryView::Album(album_view) => album_view.clone().into_any_element(),
                        LibraryView::Release(release_view) => {
                            release_view.clone().into_any_element()
                        }
                        LibraryView::Playlist(playlist_view) => {
                            playlist_view.clone().into_any_element()
                        }
                    }),
            )
    }
}
