use std::sync::Arc;

use gpui::{App, SharedString};

use crate::ui::components::palette::{FinderItemLeft, PaletteItem};

#[derive(Debug, Clone, PartialEq)]
pub struct AlbumPaletteItem {
    pub id: u32,
    pub title: String,
    pub artist: String,
}

impl AlbumPaletteItem {
    pub fn new(id: u32, title: String, artist: String) -> Self {
        Self { id, title, artist }
    }

    pub fn from_search_results(results: Vec<(u32, String, String)>) -> Vec<Arc<AlbumPaletteItem>> {
        results
            .into_iter()
            .map(|(id, title, artist)| Arc::new(AlbumPaletteItem::new(id, title, artist)))
            .collect()
    }

    pub fn thumbnail_path(&self) -> String {
        format!("!db://album/{}/thumb", self.id)
    }
}

impl PaletteItem for AlbumPaletteItem {
    fn left_content(&self, _cx: &mut App) -> Option<FinderItemLeft> {
        Some(FinderItemLeft::Image(self.thumbnail_path().into()))
    }

    fn middle_content(&self, _cx: &mut App) -> SharedString {
        self.title.clone().into()
    }

    fn right_content(&self, _cx: &mut App) -> Option<SharedString> {
        Some(self.artist.clone().into())
    }
}
