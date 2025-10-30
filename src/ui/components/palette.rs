mod finder;

use std::{marker::PhantomData, sync::Arc};

use gpui::{App, AppContext, Entity, FocusHandle};
use nucleo::Utf32String;

use crate::ui::components::{input::TextInput, palette::finder::Finder};

pub struct Palette<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    input: Entity<TextInput>,
    handle: FocusHandle,
    finder: Entity<Finder<T, MatcherFunc, OnAccept>>,
}

impl<T, MatcherFunc, OnAccept> Palette<T, MatcherFunc, OnAccept>
where
    T: Send + Sync + PartialEq + 'static,
    MatcherFunc: Fn(&Arc<T>, &mut App) -> Utf32String + 'static,
    OnAccept: Fn(&Arc<T>, &mut App) + 'static,
{
    pub fn new(
        cx: &mut App,
        items: Vec<Arc<T>>,
        matcher: MatcherFunc,
        on_accept: OnAccept,
    ) -> Entity<Self> {
        let handle = cx.focus_handle();
        let input = TextInput::new(cx, handle.clone(), None, None, None);

        cx.new(|cx| Palette {
            input,
            handle,
            finder: Finder::new(cx, items, matcher, on_accept),
        })
    }
}
