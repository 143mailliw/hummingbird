use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div, px,
};
use nucleo::Utf32String;

use crate::{
    library::{
        db::LibraryAccess,
        types::{PlaylistType, PlaylistWithCount},
    },
    ui::{
        components::{
            icons::{PLAYLIST, STAR_FILLED},
            modal::modal,
            palette::{FinderItemLeft, Palette, PaletteItem},
        },
        models::{Models, PlaylistEvent},
    },
};

// i64 here is the track ID
impl PaletteItem for (i64, PlaylistWithCount) {
    fn left_content(&self, _: &mut App) -> Option<FinderItemLeft> {
        Some(FinderItemLeft::Icon(match self.1.playlist_type {
            PlaylistType::User => PLAYLIST.into(),
            PlaylistType::System => STAR_FILLED.into(),
        }))
    }

    fn middle_content(&self, cx: &mut App) -> SharedString {
        let has_track = cx.playlist_has_track(self.1.id, self.0).ok().flatten();

        if has_track.is_none() {
            format!("Add to {}", self.1.name).into()
        } else {
            format!("Remove from {}", self.1.name).into()
        }
    }

    fn right_content(&self, _: &mut App) -> Option<SharedString> {
        None
    }
}

type MatcherFunc = Box<dyn Fn(&Arc<(i64, PlaylistWithCount)>, &mut App) -> Utf32String + 'static>;
type OnAccept = Box<dyn Fn(&Arc<(i64, PlaylistWithCount)>, &mut App) + 'static>;

pub struct AddToPlaylist {
    show: Entity<bool>,
    palette: Entity<Palette<(i64, PlaylistWithCount), MatcherFunc, OnAccept>>,
}

impl AddToPlaylist {
    pub fn new(cx: &mut App, show: Entity<bool>, track_id: i64) -> Entity<Self> {
        cx.new(|cx| {
            cx.observe(&show, |this: &mut Self, _, cx| {
                this.palette.update(cx, |this, cx| {
                    this.reset(cx);
                });

                cx.notify();
            })
            .detach();

            let matcher: MatcherFunc = Box::new(|playlist, _| playlist.1.name.0.to_string().into());

            let show_clone = show.clone();

            let on_accept: OnAccept = Box::new(move |playlist, cx| {
                let has_track = cx
                    .playlist_has_track(playlist.1.id, track_id)
                    .ok()
                    .flatten();

                if let Some(id) = has_track {
                    cx.remove_playlist_item(id).unwrap();
                } else {
                    cx.add_playlist_item(playlist.1.id, track_id).unwrap();
                }

                let playlist_tracker = cx.global::<Models>().playlist_tracker.clone();

                playlist_tracker.update(cx, |_, cx| {
                    cx.emit(PlaylistEvent::PlaylistUpdated(playlist.1.id));
                });

                show_clone.write(cx, false);
            });

            let items = (*cx.get_all_playlists().unwrap())
                .clone()
                .into_iter()
                .map(|playlist| (track_id, playlist))
                .map(Arc::new)
                .collect();

            Self {
                show,
                palette: Palette::new(cx, items, matcher, on_accept),
            }
        })
    }
}

impl Render for AddToPlaylist {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show = self.show.clone();
        let palette = self.palette.clone();
        let show_read = *self.show.read(cx);

        if show_read {
            cx.update_entity(&palette, |palette, _| {
                palette.focus(window);
            });

            modal()
                .child(div().w(px(550.0)).h(px(300.0)).child(palette.clone()))
                .on_exit(move |_, cx| {
                    show.update(cx, |show, cx| {
                        *show = false;
                        cx.update_entity(&palette, |palette, cx| {
                            palette.reset(cx);
                        });
                        cx.notify();
                    })
                })
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
