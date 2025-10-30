use std::sync::Arc;

use gpui::{App, AppContext, Context, Entity, EventEmitter, IntoElement, Render, Window};
use nucleo::Utf32String;
use tracing::debug;

use crate::{
    library::{db::LibraryAccess, scan::ScanEvent},
    ui::{
        components::{input::EnrichedInputAction, palette::Palette},
        library::ViewSwitchMessage,
        models::Models,
    },
};

use super::album_item::AlbumPaletteItem;

type MatcherFunc = Box<dyn Fn(&Arc<AlbumPaletteItem>, &mut App) -> Utf32String + 'static>;
type OnAccept = Box<dyn Fn(&Arc<AlbumPaletteItem>, &mut App) + 'static>;

pub struct SearchModel {
    palette: Entity<Palette<AlbumPaletteItem, MatcherFunc, OnAccept>>,
}

impl SearchModel {
    pub fn new(cx: &mut App) -> Entity<SearchModel> {
        cx.new(|cx| {
            let albums = match cx.list_albums_search() {
                Ok(album_data) => AlbumPaletteItem::from_search_results(album_data),
                Err(e) => {
                    debug!("Failed to load albums for search: {:?}", e);
                    Vec::new()
                }
            };

            let weak_self = cx.weak_entity();

            let matcher: MatcherFunc =
                Box::new(|album, _| Utf32String::from(format!("{} {}", album.title, album.artist)));

            let on_accept: OnAccept = Box::new(move |album, cx| {
                let event = ViewSwitchMessage::Release(album.id as i64);

                if let Some(search_model) = weak_self.upgrade() {
                    search_model.update(cx, |_: &mut SearchModel, cx| {
                        cx.emit(event);
                    });
                }
            });

            let palette = Palette::new(cx, albums, matcher, on_accept);

            let search_model = SearchModel { palette };

            let scan_status = cx.global::<Models>().scan_state.clone();
            let palette_weak = search_model.palette.downgrade();

            cx.observe(&scan_status, move |_, scan_event, cx| {
                let state = scan_event.read(cx);

                if *state == ScanEvent::ScanCompleteIdle
                    || *state == ScanEvent::ScanCompleteWatching
                {
                    debug!("Scan complete, refreshing album list for search");

                    let new_albums = match cx.list_albums_search() {
                        Ok(album_data) => AlbumPaletteItem::from_search_results(album_data),
                        Err(e) => {
                            debug!("Failed to reload albums after scan: {:?}", e);
                            return;
                        }
                    };

                    if let Some(palette) = palette_weak.upgrade() {
                        palette.update(cx, |_, cx| {
                            cx.emit(new_albums);
                        });
                    }
                }
            })
            .detach();

            search_model
        })
    }

    pub fn reset(&mut self, cx: &mut Context<Self>) {
        cx.update_entity(&self.palette, |palette, cx| {
            palette.reset(cx);
        });
    }

    pub fn focus(&self, window: &mut Window, cx: &Context<Self>) {
        self.palette.read(cx).focus(window);
    }
}

impl EventEmitter<String> for SearchModel {}
impl EventEmitter<ViewSwitchMessage> for SearchModel {}
impl EventEmitter<EnrichedInputAction> for SearchModel {}

impl Render for SearchModel {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.palette.clone()
    }
}
