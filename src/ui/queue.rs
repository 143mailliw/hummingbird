use crate::{
    data::{interface::GPUIDataInterface, types::UIQueueItem},
    playback::interface::GPUIPlaybackInterface,
};
use ahash::AHashMap;
use gpui::*;
use prelude::FluentBuilder;

use super::{
    components::button::{button, ButtonSize, ButtonStyle},
    constants::FONT_AWESOME,
    models::{Models, PlaybackInfo},
    theme::Theme,
    util::{create_or_retrieve_view, prune_views},
};

pub struct QueueItem {
    item: Option<UIQueueItem>,
    path: String,
    current_track: Entity<Option<String>>,
    idx: usize,
}

impl QueueItem {
    pub fn new(cx: &mut App, path: String, idx: usize, clear_cache: bool) -> Entity<Self> {
        cx.new_view(move |cx| {
            let current_track = cx.global::<PlaybackInfo>().current_track.clone();

            let interface = cx.global::<GPUIDataInterface>();

            if clear_cache {
                interface.evict_cache();
            }

            interface.get_metadata(path.clone());

            let queue_model = cx.global::<Models>().queue.clone();

            cx.subscribe(&queue_model, move |this: &mut QueueItem, _, ev, cx| {
                if ev.file_path == this.path {
                    this.item = Some(ev.clone());
                    cx.notify();
                }
            })
            .detach();

            Self {
                item: None,
                path,
                current_track,
                idx,
            }
        })
    }
}

impl Render for QueueItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        if let Some(item) = self.item.as_ref() {
            let is_current = self
                .current_track
                .read(cx)
                .as_ref()
                .map(|v| v == &item.file_path)
                .unwrap_or(false);

            let album_art = item
                .album_art
                .as_ref()
                .map(|v| ImageSource::Render(v.clone()));

            let idx = self.idx;

            div()
                .w_full()
                .id(ElementId::View(cx.entity_id()))
                .flex()
                .overflow_x_hidden()
                .gap(px(11.0))
                .h(px(59.0))
                .p(px(11.0))
                .border_b(px(1.0))
                .cursor_pointer()
                .border_color(theme.border_color)
                .when(is_current, |div| div.bg(theme.queue_item_current))
                .on_click(move |_, _, cx| {
                    cx.global::<GPUIPlaybackInterface>().jump(idx);
                })
                .hover(|div| div.bg(theme.queue_item_hover))
                .active(|div| div.bg(theme.queue_item_active))
                .child(
                    div()
                        .id("album-art")
                        .rounded(px(4.0))
                        .bg(theme.album_art_background)
                        .shadow_sm()
                        .w(px(36.0))
                        .h(px(36.0))
                        .flex_shrink_0()
                        .when(album_art.is_some(), |div| {
                            div.child(
                                img(album_art.unwrap())
                                    .w(px(36.0))
                                    .h(px(36.0))
                                    .rounded(px(4.0)),
                            )
                        }),
                )
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .line_height(rems(1.0))
                        .text_size(px(15.0))
                        .gap_1()
                        .overflow_x_hidden()
                        .child(
                            div()
                                .text_ellipsis()
                                .font_weight(FontWeight::EXTRA_BOLD)
                                .child(item.track_name.clone()), // .child(item.metadata.name.clone().unwrap_or(
                                                                 //     item.file_path.split(MAIN_SEPARATOR).last().unwrap().into(),
                                                                 // )),
                        )
                        .child(div().text_ellipsis().child(item.artist_name.clone())),
                )
        } else {
            // TODO: Skeleton for this
            div()
                .h(px(59.0))
                .border_t(px(1.0))
                .border_color(theme.border_color)
                .w_full()
                .id(ElementId::View(cx.entity_id()))
        }
    }
}

pub struct Queue {
    views_model: Entity<AHashMap<usize, Entity<QueueItem>>>,
    render_counter: Entity<usize>,
    state: ListState,
    shuffling: Entity<bool>,
    show_queue: Entity<bool>,
}

impl Queue {
    pub fn new(cx: &mut App, show_queue: Entity<bool>) -> Entity<Self> {
        cx.new(|cx| {
            let views_model = cx.new(|_| AHashMap::new());
            let render_counter = cx.new(|_| 0);
            let items = cx.global::<Models>().queue.clone();

            cx.observe(&items, move |this: &mut Queue, m, cx| {
                this.views_model = cx.new(|_| AHashMap::new());
                this.render_counter = cx.new(|_| 0);

                let items = m.read(cx).clone();
                let views_model = this.views_model.clone();
                let render_counter = this.render_counter.clone();

                this.state = ListState::new(
                    items.0.len(),
                    ListAlignment::Top,
                    px(200.0),
                    move |idx, _, cx| {
                        let item = items.0.get(idx).unwrap().clone();
                        let was_removed =
                            prune_views(views_model.clone(), render_counter.clone(), idx, cx);

                        div()
                            .child(create_or_retrieve_view(
                                views_model.clone(),
                                idx,
                                move |cx| QueueItem::new(cx, item, idx, was_removed),
                                cx,
                            ))
                            .into_any_element()
                    },
                );

                cx.notify();
            })
            .detach();

            let shuffling = cx.global::<PlaybackInfo>().shuffling.clone();

            cx.observe(&shuffling, |_, _, cx| {
                cx.notify();
            })
            .detach();

            Self {
                views_model,
                render_counter,
                state: ListState::new(0, ListAlignment::Top, px(200.0), move |_, _, _| {
                    div().into_any_element()
                }),
                shuffling,
                show_queue,
            }
        })
    }
}

impl Render for Queue {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let shuffling = self.shuffling.read(cx);

        div()
            // .absolute()
            // .top_0()
            // .right_0()
            .h_full()
            .min_w(px(275.0))
            .max_w(px(275.0))
            .w(px(275.0))
            .border_l(px(1.0))
            .flex_shrink_0()
            .border_color(theme.border_color)
            .pb(px(0.0))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .border_b_1()
                    .border_color(theme.border_color)
                    .child(
                        div()
                            .flex()
                            .w_full()
                            .child(
                                div()
                                    .ml(px(12.0))
                                    .pt(px(5.0))
                                    .flex()
                                    .font_weight(FontWeight::BOLD)
                                    .child(div().text_sm().child("Queue")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .id("back")
                                    .font_family(FONT_AWESOME)
                                    .pr(px(12.0))
                                    .pl(px(12.0))
                                    .py(px(5.0))
                                    .ml_auto()
                                    .text_sm()
                                    .border_l_1()
                                    .border_color(theme.border_color)
                                    .hover(|this| this.bg(theme.nav_button_hover))
                                    .active(|this| this.bg(theme.nav_button_active))
                                    .cursor_pointer()
                                    .child("")
                                    .on_click(cx.listener(|this: &mut Self, _, cx| {
                                        this.show_queue.update(cx, |v, _| *v = !(*v))
                                    })),
                            ),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .pt(px(24.0))
                    .pb(px(12.0))
                    .px(px(12.0))
                    .flex()
                    .child(
                        div()
                            .line_height(px(26.0))
                            .font_weight(FontWeight::BOLD)
                            .text_size(px(26.0))
                            .child("Queue"),
                    ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .border_t_1()
                    .border_b_1()
                    .border_color(theme.border_color)
                    .child(
                        button()
                            .style(ButtonStyle::MinimalNoRounding)
                            .size(ButtonSize::Large)
                            .child(div().font_family(FONT_AWESOME).child(""))
                            .child("Clear")
                            .w_full()
                            .id("clear-queue")
                            .on_click(|_, _, cx| {
                                cx.global::<GPUIPlaybackInterface>().clear_queue();
                                cx.global::<GPUIPlaybackInterface>().stop();
                            }),
                    )
                    .child(
                        button()
                            .style(ButtonStyle::MinimalNoRounding)
                            .size(ButtonSize::Large)
                            .child(div().font_family(FONT_AWESOME).child(""))
                            .when(*shuffling, |this| this.child("Shuffling"))
                            .when(!shuffling, |this| this.child("Shuffle"))
                            .w_full()
                            .id("queue-shuffle")
                            .on_click(|_, _, cx| {
                                cx.global::<GPUIPlaybackInterface>().toggle_shuffle()
                            }),
                    ),
            )
            .child(list(self.state.clone()).w_full().h_full().flex().flex_col())
    }
}
