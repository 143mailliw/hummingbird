use std::{any::Any, sync::Arc};

use gpui::{
    Action, App, AppContext, Context, Entity, IntoElement, ParentElement, Render, SharedString,
    Styled, Window, actions, div, px,
};
use nucleo::Utf32String;
use rustc_hash::FxHashMap;
use std::hash::Hash;

use crate::ui::{
    components::{
        modal::modal,
        palette::{FinderItemLeft, Palette, PaletteItem},
    },
    global_actions::{About, ForceScan, Next, PlayPause, Previous, Quit, Search},
};

actions!(hummingbird, [OpenPalette]);

struct Command {
    category: Option<SharedString>,
    name: SharedString,
    action: Box<dyn Action + Send + Sync>,
}

impl Command {
    fn new(
        category: Option<impl Into<SharedString>>,
        name: impl Into<SharedString>,
        action: impl Action + Send + Sync,
    ) -> Arc<Self> {
        Arc::new(Command {
            category: category.map(Into::into),
            name: name.into(),
            action: Box::new(action),
        })
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.action.partial_eq(&(*other.action))
    }
}

impl Hash for Command {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.action.name().hash(state);
    }
}

impl PaletteItem for Command {
    fn left_content(
        &self,
        _: &mut gpui::App,
    ) -> Option<super::components::palette::FinderItemLeft> {
        self.category
            .clone()
            .map(|category| FinderItemLeft::Text(category))
    }

    fn middle_content(&self, _: &mut gpui::App) -> SharedString {
        self.name.clone()
    }

    fn right_content(&self, cx: &mut gpui::App) -> Option<SharedString> {
        cx.key_bindings()
            .borrow()
            .bindings_for_action(&(*self.action))
            .last()
            .map(|binding| {
                binding
                    .keystrokes()
                    .iter()
                    .map(|key| key.to_string())
                    .collect::<Vec<String>>()
                    .join(" + ")
                    .into()
            })
    }
}

type MatcherFunc = Box<dyn Fn(&Arc<Command>, &mut App) -> Utf32String + 'static>;
type OnAccept = Box<dyn Fn(&Arc<Command>, &mut App) + 'static>;

pub struct CommandPalette {
    show: bool,
    palette: Entity<Palette<Command, MatcherFunc, OnAccept>>,
    items: FxHashMap<&'static str, Arc<Command>>,
}

impl CommandPalette {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<Self> {
        cx.new(|cx| {
            let matcher: MatcherFunc = Box::new(|item, _| item.name.to_string().into());

            let weak_self = cx.weak_entity();
            let on_accept: OnAccept = Box::new(move |item, cx| {
                cx.dispatch_action(&(*item.action));
                weak_self
                    .update(cx, |this: &mut Self, cx| {
                        this.show = false;
                        cx.notify();
                    })
                    .ok();
            });

            let mut items = FxHashMap::default();

            // add basic items
            items.insert(
                "hummingbird::quit",
                Command::new(Some("Hummingbird"), "Quit", Quit),
            );
            items.insert(
                "hummingbird::about",
                Command::new(Some("Hummingbird"), "About", About),
            );
            items.insert(
                "hummingbird::search",
                Command::new(Some("Hummingbird"), "Search", Search),
            );

            items.insert(
                "player::playpause",
                Command::new(Some("Playback"), "Pause/Resume Current Track", PlayPause),
            );
            items.insert(
                "player::next",
                Command::new(Some("Playback"), "Next Track", Next),
            );
            items.insert(
                "player::previous",
                Command::new(Some("Playback"), "Previous Track", Previous),
            );

            items.insert(
                "scan::forcescan",
                Command::new(Some("Scan"), "Rescan Entire Library", ForceScan),
            );

            let palette = Palette::new(
                cx,
                items.iter().map(|v| v.1.clone()).collect(),
                matcher,
                on_accept,
            );

            let weak_self = cx.weak_entity();
            App::on_action(cx, move |_: &OpenPalette, cx: &mut App| {
                weak_self
                    .update(cx, |this: &mut Self, cx| {
                        this.show = true;
                        cx.notify();
                    })
                    .ok();
            });

            Self {
                show: false,
                items,
                palette,
            }
        })
    }
}

impl Render for CommandPalette {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.show {
            let palette = self.palette.clone();
            let weak_self = cx.weak_entity();

            palette.update(cx, |palette, cx| {
                palette.focus(window);
            });

            modal()
                .child(div().w(px(550.0)).h(px(300.0)).child(palette.clone()))
                .on_exit(move |_, cx| {
                    weak_self
                        .update(cx, |this, cx| {
                            this.show = false;
                            cx.notify();
                        })
                        .ok();
                })
                .into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
