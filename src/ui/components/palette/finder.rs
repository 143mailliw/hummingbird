use std::{
    marker::PhantomData,
    ops::{AddAssign, SubAssign},
    sync::Arc,
    time::Duration,
};

use async_channel::bounded;
use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, AppContext, Context, Div, ElementId, Entity,
    EventEmitter, InteractiveElement, IntoElement, ListAlignment, ListState, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Window,
};
use nucleo::{
    pattern::{CaseMatching, Normalization},
    Config, Nucleo, Utf32String,
};
use rustc_hash::FxHashMap;
use tracing::debug;

use crate::ui::{components::input::EnrichedInputAction, theme::Theme};

pub struct Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    query: String,
    matcher: Nucleo<Arc<T>>,
    views_model: Entity<FxHashMap<usize, Entity<FinderItem>>>,
    render_counter: Entity<usize>,
    last_match: Vec<Arc<T>>,
    list_state: ListState,
    current_selection: Entity<usize>,
    matcher_phantom: PhantomData<MatcherFunc>,
    on_accept_phantom: PhantomData<OnAccept>,
}

impl<T, MatcherFunc, OnAccept> Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + 'static,
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

            let config = Config::DEFAULT;

            let (rx, tx) = bounded(10);
            let notify = Arc::new(move || {
                rx.send_blocking(());
            });

            let views_model = cx.new(|_| FxHashMap::default());
            let render_counter = cx.new(|_| 0);

            cx.spawn(async move |weak, cx| loop {
                let mut did_regenerate = false;

                while tx.try_recv().is_ok() {
                    did_regenerate = true;
                    weak.update(cx, |this: &mut Self, cx| {
                        debug!("Received notification, regenerating list state");
                        this.regenerate_list_state(cx);
                        cx.notify();
                    })
                    .expect("unable to update weak search model");
                }

                weak.update(cx, |this: &mut Self, cx| {
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

            cx.subscribe(
                &cx.entity(),
                move |this, _, ev: &EnrichedInputAction, cx| {
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
                            let item = this.last_match.get(*idx).unwrap();

                            on_accept(item, cx);
                        }
                    }
                },
            )
            .detach();

            let get_item_display_clone = get_item_display.clone();

            cx.subscribe(&cx.entity(), move |this, _, items: &Vec<Arc<T>>, cx| {
                this.matcher.restart(false);
                let injector = this.matcher.injector();

                for item in items {
                    injector.push(item.clone(), |v, dest| {
                        dest[0] = get_item_display_clone(v, cx);
                    });
                }

                cx.notify();
            })
            .detach();

            let matcher = Nucleo::new(config, notify, None, 1);
            let injector = matcher.injector();

            for item in items {
                injector.push(item, |v, dest| {
                    dest[0] = get_item_display(v, cx);
                });
            }

            let current_selection = cx.new(|_| 0);

            Self {
                query: String::new(),
                matcher,
                views_model,
                last_match: Vec::with_capacity(0),
                render_counter,
                current_selection,
                list_state: Self::make_list_state(None),
                matcher_phantom: PhantomData,
                on_accept_phantom: PhantomData,
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

    fn get_matches(&self) -> Vec<Arc<T>> {
        let snapshot = self.matcher.snapshot();
        snapshot
            .matched_items(..100.min(snapshot.matched_item_count()))
            .map(|item| item.data.clone())
            .collect()
    }

    fn regenerate_list_state(&mut self, cx: &mut Context<Self>) {
        debug!("Regenerating list state");
        let curr_scroll = self.list_state.logical_scroll_top();
        let matches = self.get_matches();
        self.views_model = cx.new(|_| FxHashMap::default());
        self.render_counter = cx.new(|_| 0);

        self.list_state = Self::make_list_state(Some(matches));

        self.list_state.scroll_to(curr_scroll);

        cx.notify();
    }

    fn make_list_state(matches: Option<Vec<Arc<T>>>) -> ListState {
        match matches {
            Some(matches) => ListState::new(matches.len(), ListAlignment::Top, px(300.0)),
            None => ListState::new(0, ListAlignment::Top, px(64.0)),
        }
    }
}

impl<T, MatcherFunc, OnAccept> EventEmitter<String> for Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
}

impl<T, MatcherFunc, OnAccept> EventEmitter<Vec<Arc<T>>> for Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
}

impl<T, MatcherFunc, OnAccept> EventEmitter<EnrichedInputAction>
    for Finder<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
}

pub struct FinderItem {
    id: ElementId,
    left: Option<FinderItemLeft>,
    middle: SharedString,
    right: Option<SharedString>,
    idx: usize,
    current_selection: usize,
}

#[derive(Clone)]
pub enum FinderItemLeft {
    Text(SharedString),
    Icon(SharedString),
    Image(SharedString),
}

impl FinderItem {
    pub fn new(
        cx: &mut App,
        id: impl Into<ElementId>,
        left: Option<FinderItemLeft>,
        middle: SharedString,
        right: Option<SharedString>,
        idx: usize,
        current_selection: &Entity<usize>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            cx.observe(
                current_selection,
                |this: &mut Self, m, cx: &mut Context<Self>| {
                    this.current_selection = *m.read(cx);
                    cx.notify();
                },
            )
            .detach();

            Self {
                id: id.into(),
                left,
                middle,
                right,
                idx,
                current_selection: *current_selection.read(cx),
            }
        })
    }
}

impl Render for FinderItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .px(px(8.0))
            .py(px(8.0))
            .flex()
            .cursor_pointer()
            .id(self.id.clone())
            .hover(|this| this.bg(theme.palette_item_hover))
            .active(|this| this.bg(theme.palette_item_active))
            .when(self.current_selection == self.idx, |this| {
                this.bg(theme.palette_item_hover)
            })
            .when_some(self.left.clone(), |div, left| div.child("a"))
            .child(self.middle.clone())
            .when_some(self.right.clone(), |div, right| div.child(right))
    }
}
