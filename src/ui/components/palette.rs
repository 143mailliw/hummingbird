mod finder;

pub use finder::{FinderItemLeft, PaletteItem};

use std::sync::Arc;

use gpui::{
    App, AppContext, Context, Entity, EventEmitter, FocusHandle, IntoElement, ParentElement,
    Render, Styled, Window, div, px,
};
use nucleo::Utf32String;

use crate::ui::components::{
    input::{EnrichedInputAction, TextInput},
    palette::finder::Finder,
};
use crate::ui::theme::Theme;

pub struct Palette<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    input: Entity<TextInput>,
    handle: FocusHandle,
    finder: Entity<Finder<T, MatcherFunc, OnAccept>>,
}

impl<T, MatcherFunc, OnAccept> Palette<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    pub fn new(
        cx: &mut App,
        items: Vec<Arc<T>>,
        matcher: MatcherFunc,
        on_accept: OnAccept,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let handle = cx.focus_handle();
            let finder = Finder::new(cx, items, matcher, on_accept);
            let finder_clone = finder.clone();

            let handler = move |action, _: &mut Window, cx: &mut App| {
                cx.update_entity(&finder_clone, |_, cx| {
                    cx.emit(action);
                });
            };

            let input = TextInput::new(cx, handle.clone(), None, None, Some(Box::new(handler)));

            // Connect input changes to finder
            cx.subscribe(&input, move |this: &mut Self, _, ev: &String, cx| {
                cx.update_entity(&this.finder, |_, cx| {
                    cx.emit(ev.clone());
                });
            })
            .detach();

            // Forward item list updates to finder
            cx.subscribe(
                &cx.entity(),
                move |this: &mut Self, _, items: &Vec<Arc<T>>, cx| {
                    cx.update_entity(&this.finder, |_, cx| {
                        cx.emit(items.clone());
                    });
                },
            )
            .detach();

            Palette {
                input,
                handle,
                finder,
            }
        })
    }

    pub fn focus(&self, window: &mut Window) {
        self.handle.focus(window);
    }

    pub fn reset(&self, cx: &mut Context<Self>) {
        cx.update_entity(&self.input, |input, cx| {
            input.reset();
            cx.notify();
        });
        cx.update_entity(&self.finder, |finder, cx| {
            finder.set_query("".to_string(), cx);
            cx.notify();
        });
    }
}

impl<T, MatcherFunc, OnAccept> Render for Palette<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .w_full()
            .h_full()
            .overflow_hidden()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .p(px(12.0))
                    .line_height(px(14.0))
                    .h(px(40.0))
                    .text_sm()
                    .border_b(px(1.0))
                    .border_color(theme.border_color)
                    .child(self.input.clone()),
            )
            .child(
                div()
                    .flex()
                    .w_full()
                    .h_full()
                    // FIXME: weird layout issue, this is a hack
                    // eventually this should be removed
                    .pb(px(40.0))
                    .child(self.finder.clone()),
            )
    }
}

impl<T, MatcherFunc, OnAccept> EventEmitter<Vec<Arc<T>>> for Palette<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
}

impl<T, MatcherFunc, OnAccept> EventEmitter<EnrichedInputAction>
    for Palette<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + PaletteItem + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
}
