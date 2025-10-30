use std::{collections::VecDeque, rc::Rc};

use gpui::*;

use crate::{
    library::{
        scan::ScanEvent,
        types::{Album, table::AlbumColumn},
    },
    ui::{
        components::table::{Table, TableEvent},
        models::Models,
    },
};

use super::ViewSwitchMessage;

#[derive(Clone)]
pub struct AlbumView {
    table: Entity<Table<Album, AlbumColumn>>,
}

impl AlbumView {
    pub(super) fn new(
        cx: &mut App,
        view_switch_model: Entity<VecDeque<ViewSwitchMessage>>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let state = cx.global::<Models>().scan_state.clone();

            let handler = Rc::new(move |cx: &mut App, id: &(u32, String)| {
                view_switch_model
                    .update(cx, |_, cx| cx.emit(ViewSwitchMessage::Release(id.0 as i64)))
            });

            let table = Table::new(cx, Some(handler));

            let table_clone = table.clone();

            cx.observe(&state, move |_: &mut AlbumView, e, cx| {
                let value = e.read(cx);
                match value {
                    ScanEvent::ScanCompleteIdle => {
                        table_clone.update(cx, |_, cx| cx.emit(TableEvent::NewRows));
                    }
                    ScanEvent::ScanProgress { current, .. } => {
                        if current % 100 == 0 {
                            table_clone.update(cx, |_, cx| cx.emit(TableEvent::NewRows));
                        }
                    }
                    _ => {}
                }
            })
            .detach();

            AlbumView { table }
        })
    }
}

impl Render for AlbumView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .h_full()
            .max_w(px(1000.0))
            .pt(px(10.0))
            .pb(px(0.0))
            .child(self.table.clone())
    }
}
