use std::sync::Arc;

use gpui::{div, App, AppContext, Context, Entity, ParentElement, Render, Window};

use crate::library::{db::LibraryAccess, types::Playlist};

pub struct PlaylistView {
    playlist: Arc<Playlist>,
    playlist_track_ids: Arc<Vec<(i32,)>>,
}

impl PlaylistView {
    pub fn new(cx: &mut App, playlist_id: i64) -> Entity<Self> {
        cx.new(|cx| Self {
            playlist: cx.get_playlist(playlist_id).unwrap(),
            playlist_track_ids: cx.get_playlist_tracks(playlist_id).unwrap(),
        })
    }
}

impl Render for PlaylistView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        div().child(self.playlist.name.clone())
    }
}
