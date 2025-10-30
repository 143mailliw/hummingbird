pub mod table_data;
mod table_item;

use std::{rc::Rc, sync::Arc};

use ahash::AHashMap;
use gpui::{prelude::FluentBuilder, *};
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;
use table_data::{Column, TableData, TableSort};
use table_item::TableItem;

use crate::ui::{
    caching::hummingbird_cache,
    components::icons::{CHEVRON_DOWN, CHEVRON_UP, icon},
    theme::Theme,
    util::{create_or_retrieve_view, prune_views},
};

type RowMap<T, C> = AHashMap<usize, Entity<TableItem<T, C>>>;

#[allow(type_alias_bounds)]
pub type OnSelectHandler<T, C>
where
    C: Column,
    T: TableData<C>,
= Rc<dyn Fn(&mut App, &T::Identifier) + 'static>;

#[derive(Clone)]
pub struct Table<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    columns: Entity<Arc<IndexMap<C, f32, FxBuildHasher>>>,
    views: Entity<RowMap<T, C>>,
    render_counter: Entity<usize>,
    // list_state: ListState,
    items: Option<Arc<Vec<T::Identifier>>>,
    sort_method: Entity<Option<TableSort<C>>>,
    on_select: Option<OnSelectHandler<T, C>>,
}

pub enum TableEvent {
    NewRows,
}

impl<T, C> EventEmitter<TableEvent> for Table<T, C>
where
    T: TableData<C>,
    C: Column + 'static,
{
}

impl<T, C> Table<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    pub fn new(cx: &mut App, on_select: Option<OnSelectHandler<T, C>>) -> Entity<Self> {
        cx.new(|cx| {
            let columns = cx.new(|_| Arc::new(T::default_columns()));
            let views = cx.new(|_| AHashMap::new());
            let render_counter = cx.new(|_| 0);
            let sort_method = cx.new(|_| None);

            let items = T::get_rows(cx, None).ok().map(Arc::new);

            // let list_state = Self::make_list_state(
            //     cx,
            //     views.clone(),
            //     render_counter.clone(),
            //     &sort_method,
            //     columns.clone(),
            //     on_select.clone(),
            // );

            cx.observe(&sort_method, |this: &mut Table<T, C>, sort, cx| {
                let sort_method = *sort.read(cx);
                let items = T::get_rows(cx, sort_method).ok().map(Arc::new);

                this.views = cx.new(|_| AHashMap::new());
                this.render_counter = cx.new(|_| 0);
                this.items = items;

                cx.notify();
            })
            .detach();

            cx.subscribe(&cx.entity(), |this, _, event, cx| match event {
                TableEvent::NewRows => {
                    let sort_method = *this.sort_method.read(cx);
                    let items = T::get_rows(cx, sort_method).ok().map(Arc::new);

                    this.views = cx.new(|_| AHashMap::new());
                    this.render_counter = cx.new(|_| 0);
                    this.items = items;

                    cx.notify();
                }
            })
            .detach();

            Self {
                columns,
                views,
                render_counter,
                // list_state,
                items,
                sort_method,
                on_select,
            }
        })
    }

    // fn make_list_state(
    //     cx: &mut Context<'_, Self>,
    //     views: Entity<RowMap<T, C>>,
    //     render_counter: Entity<usize>,
    //     sort_method_entity: &Entity<Option<TableSort<C>>>,
    //     columns: Entity<Arc<IndexMap<C, f32, FxBuildHasher>>>,
    //     handler: Option<OnSelectHandler<T, C>>,
    // ) -> ListState {
    //     let sort_method = *sort_method_entity.read(cx);
    //     let Ok(rows) = T::get_rows(cx, sort_method) else {
    //         warn!("Failed to get rows");
    //         return ListState::new(0, ListAlignment::Top, px(64.0), move |_, _, _| {
    //             div().into_any_element()
    //         });
    //     };

    //     let idents_rc = Rc::new(rows);

    //     ListState::new(
    //         idents_rc.len(),
    //         ListAlignment::Top,
    //         px(300.0),
    //         move |idx, _, cx| {
    //             let idents_rc = idents_rc.clone();

    //             prune_views(&views, &render_counter, idx, cx);
    //             div()
    //                 .w_full()
    //                 .child(create_or_retrieve_view(
    //                     &views,
    //                     idx,
    //                     |cx| TableItem::new(cx, idents_rc[idx].clone(), &columns, handler.clone()),
    //                     cx,
    //                 ))
    //                 .into_any_element()
    //         },
    //     )
    // }
}

impl<T, C> Render for Table<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let mut header = div().w_full().flex();
        let theme = cx.global::<Theme>();
        let sort_method = self.sort_method.read(cx);
        let items = self.items.clone();
        let views_model = self.views.clone();
        let render_counter = self.render_counter.clone();
        let columns = self.columns.clone();
        let handler = self.on_select.clone();

        if T::has_images() {
            header = header.child(
                div()
                    .w(px(47.0))
                    .h(px(36.0))
                    .pl(px(21.0))
                    .pr(px(10.0))
                    .py(px(2.0))
                    .text_sm()
                    .flex_shrink_0()
                    .text_ellipsis()
                    // .border_r_1()
                    .border_color(theme.border_color)
                    .border_b_1()
                    .border_color(theme.border_color),
            );
        }

        for (i, column) in self.columns.read(cx).iter().enumerate() {
            let width = *column.1;
            let column_id = *column.0;
            header = header.child(
                div()
                    .flex()
                    .w(px(width))
                    .when(T::has_images(), |div| {
                        div.h(px(36.0)).px(px(12.0)).py(px(6.0))
                    })
                    .when(!T::has_images(), |div| {
                        div.h(px(30.0))
                            .px(px(10.0))
                            .py(px(2.0))
                            .when(i == 0, |div| div.pl(px(21.0)))
                    })
                    .text_sm()
                    .flex_shrink_0()
                    // .when(i != self.columns.len() - 1, |div| {
                    //     div.border_r_1().border_color(theme.border_color)
                    // })
                    .border_b_1()
                    .border_color(theme.border_color)
                    .font_weight(FontWeight::BOLD)
                    .child(SharedString::new_static(column_id.get_column_name()))
                    .when_some(sort_method.as_ref(), |this, method| {
                        this.when(method.column == column_id, |this| {
                            this.child(
                                icon(if method.ascending {
                                    CHEVRON_UP
                                } else {
                                    CHEVRON_DOWN
                                })
                                .size(px(14.0))
                                .ml(px(4.0))
                                .my_auto(),
                            )
                        })
                    })
                    .id(i)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.sort_method.update(cx, move |this, cx| {
                            if let Some(method) = this.as_mut() {
                                if method.column == column_id {
                                    method.ascending = !method.ascending;
                                } else {
                                    *this = Some(TableSort {
                                        column: column_id,
                                        ascending: true,
                                    });
                                }
                            } else {
                                *this = Some(TableSort {
                                    column: column_id,
                                    ascending: true,
                                });
                            }

                            cx.notify();
                        })
                    })),
            );
        }

        div()
            .image_cache(hummingbird_cache((T::get_table_name(), 0_usize), 100))
            .id(T::get_table_name())
            .overflow_x_scroll()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .child(
                div()
                    .w_full()
                    .pb(px(11.0))
                    .px(px(16.0))
                    .line_height(px(26.0))
                    .font_weight(FontWeight::BOLD)
                    .text_size(px(26.0))
                    .child(T::get_table_name()),
            )
            .child(header)
            .when_some(items, |this, items| {
                this.child(
                    uniform_list("table-list", items.len(), move |range, _, cx| {
                        let start = range.start;
                        let is_templ_render = range.start == 0 && range.end == 1;

                        items[range]
                            .iter()
                            .enumerate()
                            .map(|(idx, item)| {
                                let idx = idx + start;

                                if !is_templ_render {
                                    prune_views(&views_model, &render_counter, idx, cx);
                                }

                                div()
                                    .w_full()
                                    .child(create_or_retrieve_view(
                                        &views_model,
                                        idx,
                                        |cx| {
                                            TableItem::new(
                                                cx,
                                                item.clone(),
                                                &columns,
                                                handler.clone(),
                                            )
                                        },
                                        cx,
                                    ))
                                    .into_any_element()
                            })
                            .collect()
                    })
                    .w_full()
                    .h_full(),
                )
            })
    }
}
