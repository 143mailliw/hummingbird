use std::{marker::PhantomData, sync::Arc, time::Duration};

use ahash::AHashMap;
use async_channel::bounded;
use gpui::{
    App, AppContext, Context, ElementId, Entity, EventEmitter, FontWeight, InteractiveElement,
    IntoElement, ListAlignment, ListState, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, WeakEntity, Window, div, img, list, prelude::FluentBuilder,
    px,
};
use nucleo::{
    Config, Nucleo, Utf32String,
    pattern::{CaseMatching, Normalization},
};
use tracing::debug;

use crate::ui::{components::input::EnrichedInputAction, theme::Theme};

pub trait PaletteItem {
    fn left_content(&self, cx: &mut App) -> Option<FinderItemLeft>;
    fn middle_content(&self, cx: &mut App) -> SharedString;
    fn right_content(&self, cx: &mut App) -> Option<SharedString>;
}

#[derive(Clone)]
pub struct ExtraItem {
    pub left: Option<FinderItemLeft>,
    pub middle: SharedString,
    pub right: Option<SharedString>,
    pub on_accept: Arc<dyn Fn(&mut App) + Send + Sync>,
}

pub type ExtraItemProvider = Arc<dyn Fn(&str) -> Vec<ExtraItem> + Send + Sync>;

#[allow(type_alias_bounds)]
type ViewsModel<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
= Entity<AHashMap<usize, Entity<FinderItem<T, MatcherFunc, OnAccept>>>>;

pub struct Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    query: String,
    matcher: Nucleo<Arc<T>>,
    views_model: ViewsModel<T, MatcherFunc, OnAccept>,
    render_counter: Entity<usize>,
    last_match: Vec<Arc<T>>,
    extra_providers: Vec<ExtraItemProvider>,
    extra_items: Vec<ExtraItem>,
    list_state: ListState,
    current_selection: Entity<usize>,
    on_accept: Arc<OnAccept>,
    phantom: PhantomData<MatcherFunc>,
}

impl<T, MatcherFunc, OnAccept> Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    pub fn new(
        cx: &mut App,
        items: Vec<Arc<T>>,
        get_item_display: MatcherFunc,
        on_accept: OnAccept,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let get_item_display = Arc::new(get_item_display);
            let on_accept = Arc::new(on_accept);

            let config = Config::DEFAULT;

            // make notification channel
            let (sender, receiver) = bounded(10);
            let notify = Arc::new(move || {
                // if it's full it doesn't really matter, it'll already update
                _ = sender.try_send(());
            });

            let views_model = cx.new(|_| AHashMap::default());
            let render_counter = cx.new(|_| 0);

            let matcher = Nucleo::new(config, notify.clone(), None, 1);
            let injector = matcher.injector();

            for item in &items {
                let item_clone = item.clone();
                let search_text = (get_item_display)(&item_clone, cx);
                debug!("Injecting item with search text: '{}'", search_text);
                injector.push(item_clone, move |_v, dest| {
                    dest[0] = search_text.clone();
                });
            }

            let weak_self = cx.weak_entity();
            cx.spawn(async move |_, cx| {
                loop {
                    // get all the update notifications
                    // incase we got multiple
                    let mut needs_update = false;
                    while receiver.try_recv().is_ok() {
                        needs_update = true;
                    }

                    if needs_update {
                        if let Some(entity) = weak_self.upgrade() {
                            let _ = entity.update(cx, |this: &mut Self, cx| {
                                this.tick(10);

                                let matches: Vec<Arc<T>> = this.get_matches();
                                if matches != this.last_match {
                                    this.last_match = matches;
                                    this.regenerate_list_state(cx);
                                    cx.notify();
                                }
                            });
                        } else {
                            return;
                        }
                    }

                    cx.background_executor()
                        .timer(Duration::from_millis(10))
                        .await;
                }
            })
            .detach();

            // update when the query updates
            cx.subscribe(&cx.entity(), |this, _, ev: &String, cx| {
                this.set_query(ev.clone(), cx);
            })
            .detach();

            // handle keyboard navigation
            let on_accept_clone = on_accept.clone();
            cx.subscribe(
                &cx.entity(),
                move |this, _, ev: &EnrichedInputAction, cx| match ev {
                    EnrichedInputAction::Previous => {
                        this.current_selection.update(cx, |sel, cx| {
                            if *sel > 0 {
                                *sel -= 1;
                            }
                            cx.notify();
                        });

                        let idx = *this.current_selection.read(cx);
                        this.list_state.scroll_to_reveal_item(idx);
                    }
                    EnrichedInputAction::Next => {
                        let max_idx = this.list_state.item_count().saturating_sub(1);
                        this.current_selection.update(cx, |sel, cx| {
                            if *sel < max_idx {
                                *sel += 1;
                            }
                            cx.notify();
                        });

                        let idx = *this.current_selection.read(cx);
                        this.list_state.scroll_to_reveal_item(idx);
                    }
                    EnrichedInputAction::Accept => {
                        let idx = *this.current_selection.read(cx);
                        if idx < this.extra_items.len() {
                            if let Some(extra) = this.extra_items.get(idx) {
                                (extra.on_accept)(cx);
                            }
                        } else {
                            let match_idx = idx.saturating_sub(this.extra_items.len());
                            if let Some(item) = this.last_match.get(match_idx) {
                                on_accept_clone(item, cx);
                            }
                        }
                    }
                },
            )
            .detach();

            // handle item list updates
            let get_item_display_for_updates = get_item_display.clone();
            cx.subscribe(&cx.entity(), move |this, _, items: &Vec<Arc<T>>, cx| {
                this.matcher.restart(false);
                let injector = this.matcher.injector();

                for item in items {
                    let item_clone = item.clone();
                    let search_text = (get_item_display_for_updates)(&item_clone, cx);
                    injector.push(item_clone, move |_v, dest| {
                        dest[0] = search_text.clone();
                    });
                }

                cx.notify();
            })
            .detach();

            let current_selection = cx.new(|_| 0);

            Self {
                query: String::new(),
                matcher,
                views_model,
                last_match: Vec::new(),
                extra_providers: Vec::new(),
                extra_items: Vec::new(),
                render_counter,
                current_selection,
                list_state: Self::make_list_state(None),
                on_accept,
                phantom: PhantomData,
            }
        })
    }

    pub fn register_extra_provider(&mut self, provider: ExtraItemProvider, cx: &mut Context<Self>) {
        self.extra_providers.push(provider);
        self.recompute_extra_items();
        self.regenerate_list_state(cx);
    }

    fn recompute_extra_items(&mut self) {
        let mut new_items: Vec<ExtraItem> = Vec::new();
        for provider in &self.extra_providers {
            let mut provided = (provider)(&self.query);
            new_items.append(&mut provided);
        }
        self.extra_items = new_items;
    }

    pub fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        debug!("Setting query: '{}' (previous: '{}')", query, self.query);
        self.query = query.clone();

        self.matcher
            .pattern
            .reparse(0, &query, CaseMatching::Smart, Normalization::Smart, false);

        // recompute dynamic extra items based on query
        self.recompute_extra_items();

        // get some matches ready immediately
        self.tick(20);

        let matches = self.get_matches();

        // if there are extras or the items are different regenerate the list state
        if matches != self.last_match || !self.extra_items.is_empty() {
            self.last_match = matches;
            self.regenerate_list_state(cx);
        }

        self.current_selection.update(cx, |sel, cx| {
            *sel = 0;
            cx.notify();
        });
        self.list_state.scroll_to_reveal_item(0);

        cx.notify();
    }

    fn tick(&mut self, iterations: u32) {
        self.matcher.tick(iterations as u64);
    }

    fn get_matches(&self) -> Vec<Arc<T>> {
        let snapshot = self.matcher.snapshot();
        let count = snapshot.matched_item_count();
        let limit = 100.min(count);

        snapshot
            .matched_items(..limit)
            .map(|item| item.data.clone())
            .collect()
    }

    pub fn regenerate_list_state(&mut self, cx: &mut Context<Self>) {
        let matches = self.get_matches();
        let curr_scroll = self.list_state.logical_scroll_top();

        self.views_model = cx.new(|_| AHashMap::default());
        self.render_counter = cx.new(|_| 0);

        let total = matches.len() + self.extra_items.len();
        self.list_state = Self::make_list_state(Some(total));
        self.list_state.scroll_to(curr_scroll);
    }

    fn make_list_state(total_count: Option<usize>) -> ListState {
        match total_count {
            Some(count) => ListState::new(count, ListAlignment::Top, px(300.0)),
            None => ListState::new(0, ListAlignment::Top, px(64.0)),
        }
    }
}

impl<T, MatcherFunc, OnAccept> EventEmitter<String> for Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
}

impl<T, MatcherFunc, OnAccept> EventEmitter<Vec<Arc<T>>> for Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
}

impl<T, MatcherFunc, OnAccept> EventEmitter<EnrichedInputAction>
    for Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
}

impl<T, MatcherFunc, OnAccept> Render for Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use crate::ui::caching::hummingbird_cache;
        use crate::ui::util::{create_or_retrieve_view, prune_views};

        let last_match = self.last_match.clone();
        let extra_items = self.extra_items.clone();
        let views_model = self.views_model.clone();
        let render_counter = self.render_counter.clone();
        let current_selection = self.current_selection.clone();
        let weak_finder = cx.weak_entity();

        div()
            .w_full()
            .h_full()
            .image_cache(hummingbird_cache("finder-cache", 50))
            .id("finder")
            .flex()
            .p(px(4.0))
            .child(
                list(self.list_state.clone(), move |idx, _, cx| {
                    let extras_len = extra_items.len();
                    if idx < extras_len {
                        let extra = &extra_items[idx];

                        prune_views(&views_model, &render_counter, idx, cx);

                        div()
                            .w_full()
                            .child(create_or_retrieve_view(
                                &views_model,
                                idx,
                                {
                                    let current_selection = current_selection.clone();
                                    let weak_finder = weak_finder.clone();
                                    let extra = extra.clone();

                                    move |cx| {
                                        FinderItem::new_extra(
                                            cx,
                                            ("finder-extra-item", idx),
                                            idx,
                                            &current_selection,
                                            weak_finder.clone(),
                                            extra.clone(),
                                        )
                                    }
                                },
                                cx,
                            ))
                            .into_any_element()
                    } else if idx - extras_len < last_match.len() {
                        let item = &last_match[idx - extras_len];

                        prune_views(&views_model, &render_counter, idx, cx);

                        div()
                            .w_full()
                            .child(create_or_retrieve_view(
                                &views_model,
                                idx,
                                {
                                    let current_selection = current_selection.clone();
                                    let weak_finder = weak_finder.clone();
                                    let item = item.clone();

                                    move |cx| {
                                        FinderItem::new(
                                            cx,
                                            ("finder-item", idx),
                                            &item,
                                            idx,
                                            &current_selection,
                                            weak_finder.clone(),
                                            item.clone(),
                                        )
                                    }
                                },
                                cx,
                            ))
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    }
                })
                .flex()
                .flex_col()
                .gap(px(2.0))
                .w_full()
                .h_full(),
            )
    }
}

type OnAcceptOverride = Option<Arc<dyn Fn(&mut App) + Send + Sync>>;

pub struct FinderItem<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    id: ElementId,
    left: Option<FinderItemLeft>,
    middle: SharedString,
    right: Option<SharedString>,
    idx: usize,
    current_selection: usize,
    weak_parent: WeakEntity<Finder<T, MatcherFunc, OnAccept>>,
    item_data: Option<Arc<T>>,
    on_accept_override: OnAcceptOverride,
}

#[derive(Clone)]
pub enum FinderItemLeft {
    Text(SharedString),
    Icon(SharedString),
    Image(SharedString),
}

impl<T, MatcherFunc, OnAccept> FinderItem<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    pub fn new(
        cx: &mut App,
        id: impl Into<ElementId>,
        item: &Arc<T>,
        idx: usize,
        current_selection: &Entity<usize>,
        weak_parent: WeakEntity<Finder<T, MatcherFunc, OnAccept>>,
        item_data: Arc<T>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            cx.observe(
                current_selection,
                |this: &mut Self, selection_model, cx: &mut Context<Self>| {
                    this.current_selection = *selection_model.read(cx);
                    cx.notify();
                },
            )
            .detach();

            let left = item.left_content(cx);
            let middle = item.middle_content(cx);
            let right = item.right_content(cx);

            Self {
                id: id.into(),
                left,
                middle,
                right,
                idx,
                current_selection: *current_selection.read(cx),
                weak_parent,
                item_data: Some(item_data),
                on_accept_override: None,
            }
        })
    }

    pub fn new_extra(
        cx: &mut App,
        id: impl Into<ElementId>,
        idx: usize,
        current_selection: &Entity<usize>,
        weak_parent: WeakEntity<Finder<T, MatcherFunc, OnAccept>>,
        extra: ExtraItem,
    ) -> Entity<Self> {
        cx.new(|cx| {
            cx.observe(
                current_selection,
                |this: &mut Self, selection_model, cx: &mut Context<Self>| {
                    this.current_selection = *selection_model.read(cx);
                    cx.notify();
                },
            )
            .detach();

            Self {
                id: id.into(),
                left: extra.left.clone(),
                middle: extra.middle.clone(),
                right: extra.right.clone(),
                idx,
                current_selection: *current_selection.read(cx),
                weak_parent,
                item_data: None,
                on_accept_override: Some(extra.on_accept.clone()),
            }
        })
    }
}

impl<T, MatcherFunc, OnAccept> Render for FinderItem<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        let weak_parent = self.weak_parent.clone();
        let item_data = self.item_data.clone();
        let on_accept_override = self.on_accept_override.clone();

        div()
            .px(px(10.0))
            .py(px(6.0))
            .flex()
            .flex_row()
            .items_center()
            .cursor_pointer()
            .id(self.id.clone())
            .hover(|this| this.bg(theme.palette_item_hover))
            .active(|this| this.bg(theme.palette_item_active))
            .when(self.current_selection == self.idx, |this| {
                this.bg(theme.palette_item_hover)
            })
            .rounded(px(4.0))
            .on_click(cx.listener(move |_, _, _, cx| {
                if let Some(override_fn) = on_accept_override.clone() {
                    override_fn(cx);
                } else if let Some(parent) = weak_parent.upgrade()
                    && let Some(item) = item_data.clone()
                {
                    parent.update(cx, |finder, cx| {
                        (finder.on_accept)(&item, cx);
                    });
                }
            }))
            .when_some(self.left.clone(), |div_outer, left| {
                div_outer.child(match left {
                    FinderItemLeft::Text(text) => div()
                        .child(text)
                        .text_ellipsis()
                        .text_sm()
                        .text_color(theme.text_secondary)
                        .mr(px(4.0)),
                    FinderItemLeft::Icon(icon_name) => {
                        use crate::ui::components::icons::icon;
                        div()
                            .child(icon(icon_name).w(px(16.0)).h(px(16.0)))
                            .mr(px(8.0))
                    }
                    FinderItemLeft::Image(image_path) => div()
                        .rounded(px(2.0))
                        .bg(theme.album_art_background)
                        .shadow_sm()
                        .w(px(16.0))
                        .h(px(16.0))
                        .flex_shrink_0()
                        .mr(px(8.0))
                        .child(img(image_path).w(px(16.0)).h(px(16.0)).rounded(px(2.0))),
                })
            })
            .child(
                div()
                    .flex_shrink()
                    .font_weight(FontWeight::BOLD)
                    .text_sm()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(self.middle.clone()),
            )
            .when_some(self.right.clone(), |div_outer, right| {
                div_outer.child(
                    div()
                        .ml_auto()
                        .pl(px(8.0))
                        .flex_shrink()
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_sm()
                        .text_color(theme.text_secondary)
                        .child(right),
                )
            })
    }
}
