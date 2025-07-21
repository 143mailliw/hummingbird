use std::{
    ops::{AddAssign, Deref, SubAssign},
    rc::Rc,
    sync::{mpsc::channel, Arc},
    time::Duration,
};

use ahash::AHashMap;
use gpui::*;
use nucleo::{
    pattern::{CaseMatching, Normalization},
    Config, Nucleo, Utf32String,
};
use prelude::FluentBuilder;
use tracing::debug;

use crate::{
    library::{
        db::{AlbumMethod, LibraryAccess},
        scan::ScanEvent,
        types::Album,
    },
    ui::{
        caching::hummingbird_cache,
        components::input::EnrichedInputAction,
        library::ViewSwitchMessage,
        models::Models,
        theme::Theme,
        util::{create_or_retrieve_view, prune_views},
    },
};

pub struct SearchModel {
    query: String,
    matcher: Nucleo<(u32, String, String)>,
    list_state: ListState,
    views_model: Entity<AHashMap<usize, Entity<AlbumSearchResult>>>,
    last_match: Vec<(u32, String, String)>,
    render_counter: Entity<usize>,
    current_selection: Entity<usize>,
}

impl SearchModel {
    pub fn new(cx: &mut App) -> Entity<SearchModel> {
        cx.new(|cx| {
            let albums = cx
                .list_albums_search()
                .expect("could not retrieve albums from db");

            let config = Config::DEFAULT;

            let (rx, tx) = channel();

            let notify = Arc::new(move || {
                debug!("Resending");
                rx.send(()).unwrap();
            });

            let views_model = cx.new(|_| AHashMap::new());
            let render_counter = cx.new(|_| 0);

            cx.spawn(async move |weak, cx| loop {
                let mut did_regenerate = false;

                while tx.try_recv().is_ok() {
                    did_regenerate = true;
                    weak.update(cx, |this: &mut SearchModel, cx| {
                        debug!("Received notification, regenerating list state");
                        this.regenerate_list_state(cx);
                        cx.notify();
                    })
                    .expect("unable to update weak search model");
                }

                weak.update(cx, |this: &mut SearchModel, cx| {
                    if !did_regenerate {
                        let matches = this.get_matches();
                        if matches != this.last_match {
                            this.last_match = matches;
                            this.regenerate_list_state(cx);
                            cx.notify();
                        }
                    }
                    this.tick();
                })
                .expect("unable to update weak search model");

                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;
            })
            .detach();

            cx.subscribe(&cx.entity(), |this, _, ev: &String, cx| {
                this.set_query(ev.clone(), cx);
            })
            .detach();

            cx.subscribe(&cx.entity(), |this, _, ev: &EnrichedInputAction, cx| {
                match ev {
                    EnrichedInputAction::Previous => {
                        this.current_selection.update(cx, |this, cx| {
                            if *this != 0 {
                                // kinda wacky but the only way I could find to do this
                                this.sub_assign(1);
                            }
                            cx.notify();
                        });

                        let idx = this.current_selection.read(cx);
                        this.list_state.scroll_to_reveal_item(*idx);
                    }
                    EnrichedInputAction::Next => {
                        let len = this.list_state.item_count();
                        this.current_selection.update(cx, |this, cx| {
                            if *this < len - 1 {
                                this.add_assign(1);
                            }
                            cx.notify();
                        });

                        let idx = this.current_selection.read(cx);
                        this.list_state.scroll_to_reveal_item(*idx);
                    }
                    EnrichedInputAction::Accept => {
                        let idx = this.current_selection.read(cx);
                        let id = this.last_match.get(*idx).unwrap().0;
                        let ev = ViewSwitchMessage::Release(id as i64);

                        cx.emit(ev);
                    }
                }
            })
            .detach();

            let matcher = Nucleo::new(config, notify, None, 1);
            let injector = matcher.injector();

            for album in albums {
                injector.push(album, |v, dest| {
                    dest[0] = Utf32String::from(format!("{} {}", v.1, v.2));
                });
            }

            let current_selection = cx.new(|_| 0);

            let scan_status = cx.global::<Models>().scan_state.clone();

            cx.observe(&scan_status, |this, ev, cx| {
                let state = ev.read(cx);

                if *state == ScanEvent::ScanCompleteIdle
                    || *state == ScanEvent::ScanCompleteWatching
                {
                    let albums = cx
                        .list_albums_search()
                        .expect("could not retrieve albums from db");

                    this.matcher.restart(false);
                    let injector = this.matcher.injector();

                    for album in albums {
                        injector.push(album, |v, dest| {
                            dest[0] = Utf32String::from(format!("{} {}", v.1, v.2));
                        });
                    }

                    cx.notify();
                }
            })
            .detach();

            SearchModel {
                query: String::new(),
                matcher,
                views_model: views_model.clone(),
                render_counter: render_counter.clone(),
                list_state: Self::make_list_state(
                    cx.weak_entity(),
                    None,
                    views_model,
                    render_counter,
                    current_selection.clone(),
                ),
                last_match: Vec::new(),
                current_selection,
            }
        })
    }

    pub fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.query = query;
        self.matcher.pattern.reparse(
            0,
            &self.query,
            CaseMatching::Smart,
            Normalization::Smart,
            false,
        );

        self.current_selection = cx.new(|_| 0);
        self.list_state.scroll_to_reveal_item(0);
    }

    fn tick(&mut self) {
        self.matcher.tick(10);
    }

    fn get_matches(&self) -> Vec<(u32, String, String)> {
        let snapshot = self.matcher.snapshot();
        snapshot
            .matched_items(..100.min(snapshot.matched_item_count()))
            .map(|item| item.data.clone())
            .collect()
    }

    fn regenerate_list_state(&mut self, cx: &mut Context<Self>) {
        debug!("Regenerating list state");
        let curr_scroll = self.list_state.logical_scroll_top();
        let album_ids = self.get_matches();
        debug!("Album IDs: {:?}", album_ids);
        self.views_model = cx.new(|_| AHashMap::new());
        self.render_counter = cx.new(|_| 0);

        self.list_state = SearchModel::make_list_state(
            cx.weak_entity(),
            Some(album_ids),
            self.views_model.clone(),
            self.render_counter.clone(),
            self.current_selection.clone(),
        );

        self.list_state.scroll_to(curr_scroll);

        cx.notify();
    }

    fn make_list_state(
        weak_self: WeakEntity<Self>,
        album_ids: Option<Vec<(u32, String, String)>>,
        views_model: Entity<AHashMap<usize, Entity<AlbumSearchResult>>>,
        render_counter: Entity<usize>,
        current_selection: Entity<usize>,
    ) -> ListState {
        match album_ids {
            Some(album_ids) => {
                let album_ids_copy = Rc::new(album_ids.clone());
                let weak_self_copy = weak_self.clone();

                ListState::new(
                    album_ids.len(),
                    ListAlignment::Top,
                    px(300.0),
                    move |idx, _, cx| {
                        let album_ids = album_ids_copy.clone();
                        let weak_self = weak_self_copy.clone();
                        let selection_clone = current_selection.clone();

                        prune_views(&views_model, &render_counter, idx, cx);
                        // TODO: error handling
                        div()
                            .w_full()
                            .child(create_or_retrieve_view(
                                &views_model,
                                idx,
                                move |cx| {
                                    AlbumSearchResult::new(
                                        cx,
                                        album_ids[idx].0 as i64,
                                        weak_self,
                                        &selection_clone,
                                        idx,
                                    )
                                },
                                cx,
                            ))
                            .into_any_element()
                    },
                )
            }
            None => ListState::new(0, ListAlignment::Top, px(64.0), move |_, _, _| {
                div().into_any_element()
            }),
        }
    }
}

impl EventEmitter<String> for SearchModel {}
impl EventEmitter<ViewSwitchMessage> for SearchModel {}
impl EventEmitter<EnrichedInputAction> for SearchModel {}

impl Render for SearchModel {
    fn render(&mut self, _: &mut Window, _: &mut Context<'_, Self>) -> impl IntoElement {
        div()
            .w_full()
            .h_full()
            .image_cache(hummingbird_cache("search-model-cache", 50))
            .id("search-model")
            .flex()
            .p(px(4.0))
            .child(list(self.list_state.clone()).gap(px(5.0)).w_full().h_full())
    }
}

struct AlbumSearchResult {
    album: Option<Arc<Album>>,
    artist: Option<Arc<String>>,
    weak_parent: WeakEntity<SearchModel>,
    current_selection: usize,
    idx: usize,
    image_path: Option<SharedString>,
}

impl AlbumSearchResult {
    fn new(
        cx: &mut App,
        id: i64,
        weak_parent: WeakEntity<SearchModel>,
        current_selection: &Entity<usize>,
        idx: usize,
    ) -> Entity<AlbumSearchResult> {
        cx.new(|cx| {
            let album = cx.get_album_by_id(id, AlbumMethod::Thumbnail).ok();
            let image_path = album
                .as_ref()
                .map(|album| SharedString::from(format!("!db://album/{}/thumb", album.id)));

            let artist = album
                .as_ref()
                .and_then(|album| cx.get_artist_name_by_id(album.artist_id).ok());

            cx.observe(
                current_selection,
                |this: &mut Self, m, cx: &mut Context<Self>| {
                    this.current_selection = *m.read(cx);
                    cx.notify();
                },
            )
            .detach();

            AlbumSearchResult {
                album,
                artist,
                weak_parent,
                current_selection: *current_selection.read(cx),
                idx,
                image_path,
            }
        })
    }
}

impl Render for AlbumSearchResult {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        if let Some(album) = self.album.as_ref() {
            div()
                .px(px(8.0))
                .py(px(8.0))
                .flex()
                .cursor_pointer()
                .id(("searchresult", album.id as u64))
                .hover(|this| this.bg(theme.palette_item_hover))
                .active(|this| this.bg(theme.palette_item_active))
                .when(self.current_selection == self.idx, |this| {
                    this.bg(theme.palette_item_hover)
                })
                .rounded(px(4.0))
                .on_click(cx.listener(|this, _, _, cx| {
                    let id = this.album.as_ref().unwrap().id;
                    let ev = ViewSwitchMessage::Release(id);

                    this.weak_parent
                        .update(cx, |_, cx| {
                            cx.emit(ev);
                        })
                        .expect("album search result exists without searchmodel");
                }))
                .child(
                    div()
                        .rounded(px(2.0))
                        .bg(theme.album_art_background)
                        .shadow_sm()
                        .w(px(18.0))
                        .h(px(18.0))
                        .flex_shrink_0()
                        .when_some(self.image_path.clone(), |div, path| {
                            div.child(img(path).w(px(18.0)).h(px(18.0)).rounded(px(2.0)))
                        }),
                )
                .child(
                    div()
                        .pl(px(8.0))
                        .mt(px(2.0))
                        .line_height(px(14.0))
                        .font_weight(FontWeight::BOLD)
                        .text_sm()
                        .child(album.title.clone()),
                )
                .when_some(self.artist.as_ref(), |this, name| {
                    this.child(
                        div()
                            .ml_auto()
                            .mt(px(2.0))
                            .line_height(px(14.0))
                            .text_sm()
                            .text_color(theme.text_secondary)
                            .child(name.deref().clone()),
                    )
                })
        } else {
            debug!("Album not found");
            div().id("badresult")
        }
    }
}
