use std::sync::Arc;

use gpui::{prelude::FluentBuilder, *};
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;

use crate::ui::theme::Theme;

use super::{
    OnSelectHandler,
    table_data::{Column, TableData},
};

#[derive(Clone)]
pub struct TableItem<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    data: Option<Vec<Option<SharedString>>>,
    columns: Arc<IndexMap<C, f32, FxBuildHasher>>,
    on_select: Option<OnSelectHandler<T, C>>,
    row: Option<Arc<T>>,
    id: Option<ElementId>,
    image_path: Option<SharedString>,
}

impl<T, C> TableItem<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    pub fn new(
        cx: &mut App,
        id: T::Identifier,
        columns: &Entity<Arc<IndexMap<C, f32, FxBuildHasher>>>,
        on_select: Option<OnSelectHandler<T, C>>,
    ) -> Entity<Self> {
        let row = T::get_row(cx, id).ok().flatten();

        let id = row.as_ref().map(|row| row.get_element_id().into());

        let columns_read = columns.read(cx).clone();

        let data = row.clone().map(|row| {
            let keys = columns_read.keys();

            keys.into_iter().map(|v| row.get_column(cx, *v)).collect()
        });

        let image_path = row.as_ref().and_then(|row| row.get_image_path());

        cx.new(|cx| {
            cx.observe(columns, |this: &mut TableItem<T, C>, m, cx| {
                this.columns = m.read(cx).clone();

                this.data = this.row.clone().map(|row| {
                    let keys = this.columns.keys();

                    keys.into_iter().map(|v| row.get_column(cx, *v)).collect()
                });

                cx.notify();
            })
            .detach();

            Self {
                data,
                image_path,
                columns: columns_read,
                on_select,
                id,
                row,
            }
        })
    }
}

impl<T, C> Render for TableItem<T, C>
where
    T: TableData<C> + 'static,
    C: Column + 'static,
{
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let row_data = self.row.clone();
        let mut row = div()
            .w_full()
            .flex()
            .id(self.id.clone().unwrap_or("bad".into()))
            .when_some(self.on_select.clone(), move |div, on_select| {
                div.on_click(move |_, _, cx| {
                    let id = row_data.as_ref().unwrap().get_table_id();
                    on_select(cx, &id)
                })
                .cursor_pointer()
                .hover(|this| this.bg(theme.nav_button_hover))
                .active(|this| this.bg(theme.nav_button_active))
            });

        if T::has_images() {
            row = row.child(
                div()
                    .w(px(47.0))
                    .h(px(36.0))
                    .text_sm()
                    .pl(px(11.0))
                    .flex_shrink_0()
                    .text_ellipsis()
                    //.border_r_1()
                    .border_color(theme.border_color)
                    .border_b_1()
                    .border_color(theme.border_color)
                    .flex()
                    .child(
                        div()
                            .m_auto()
                            .w(px(22.0))
                            .h(px(22.0))
                            .rounded(px(3.0))
                            .bg(theme.album_art_background)
                            .when_some(self.image_path.clone(), |div, image| {
                                div.child(img(image).w(px(22.0)).h(px(22.0)).rounded(px(3.0)))
                            }),
                    ),
            );
        }

        if let Some(data) = self.data.as_ref() {
            for (i, column) in data.iter().enumerate() {
                let col = self
                    .columns
                    .get_index(i)
                    .expect("data references column outside of viewed table");
                let width = *col.1;
                let monospace = T::column_monospace(*col.0);
                let column = div()
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
                    .when(monospace, |div| div.font_family("Roboto Mono"))
                    .text_sm()
                    .flex_shrink_0()
                    .overflow_hidden()
                    .text_ellipsis()
                    // .when(i != data.len() - 1, |div| {
                    //     div.border_r_1().border_color(theme.border_color)
                    // })
                    .border_b_1()
                    .border_color(theme.border_color)
                    .when_some(column.clone(), |div, string| div.child(string));

                row = row.child(column);
            }
        }

        row
    }
}
