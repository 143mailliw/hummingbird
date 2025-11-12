use std::sync::Arc;

use gpui::{
    Action, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Global, IntoElement,
    ParentElement, Render, SharedString, Styled, Window, actions, div, px,
};
use nucleo::Utf32String;
use rustc_hash::FxHashMap;
use std::hash::Hash;
use tracing::error;

use crate::ui::{
    components::{
        modal::modal,
        palette::{FinderItemLeft, Palette, PaletteItem},
    },
    global_actions::{About, ForceScan, Next, PlayPause, Previous, Quit, Search},
};

actions!(hummingbird, [OpenPalette]);

pub struct Command {
    category: Option<SharedString>,
    name: SharedString,
    action: Box<dyn Action + Send + Sync>,
    focus_handle: Option<FocusHandle>,
}

impl Command {
    pub fn new(
        category: Option<impl Into<SharedString>>,
        name: impl Into<SharedString>,
        action: impl Action + Send + Sync,
        focus_handle: Option<FocusHandle>,
    ) -> Arc<Self> {
        Arc::new(Command {
            category: category.map(Into::into),
            name: name.into(),
            action: Box::new(action),
            focus_handle,
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
    items: FxHashMap<(&'static str, i64), Arc<Command>>,
}

impl CommandPalette {
    pub fn new(cx: &mut App, _: &mut Window) -> Entity<Self> {
        cx.new(|cx| {
            let matcher: MatcherFunc = Box::new(|item, _| item.name.to_string().into());

            let weak_self = cx.weak_entity();
            let on_accept: OnAccept = Box::new(move |item, cx| {
                if let Some(focus_handle) = &item.focus_handle {
                    if let Err(err) =
                        cx.update_window(cx.active_window().unwrap(), |_, window, _| {
                            focus_handle.focus(window);
                        })
                    {
                        error!("Failed to focus window, action may not trigger: {}", err);
                    }
                }
                
                cx.dispatch_action(&(*item.action));
                
                weak_self
                    .update(cx, |this: &mut Self, cx| {
                        this.show = false;
                        cx.notify();
                    })
                    .ok();
            });

            let mut items = FxHashMap::default();

            cx.subscribe_self(move |this: &mut Self, ev, cx| {
                match ev {
                    CommandEvent::NewCommand(id, command) => {
                        this.items.insert(*id, command.clone())
                    }
                    CommandEvent::RemoveCommand(id) => this.items.remove(id),
                };

                let vec: Vec<_> = this.items.values().cloned().collect();

                this.palette.update(cx, |_, cx| {
                    cx.emit(vec);
                });

                cx.notify();
            })
            .detach();

            // add basic items
            items.insert(
                ("hummingbird::quit", 0),
                Command::new(Some("Hummingbird"), "Quit", Quit, None),
            );
            items.insert(
                ("hummingbird::about", 0),
                Command::new(Some("Hummingbird"), "About", About, None),
            );
            items.insert(
                ("hummingbird::search", 0),
                Command::new(Some("Hummingbird"), "Search", Search, None),
            );

            items.insert(
                ("player::playpause", 0),
                Command::new(
                    Some("Playback"),
                    "Pause/Resume Current Track",
                    PlayPause,
                    None,
                ),
            );
            items.insert(
                ("player::next", 0),
                Command::new(Some("Playback"), "Next Track", Next, None),
            );
            items.insert(
                ("player::previous", 0),
                Command::new(Some("Playback"), "Previous Track", Previous, None),
            );

            items.insert(
                ("scan::forcescan", 0),
                Command::new(Some("Scan"), "Rescan Entire Library", ForceScan, None),
            );

            let palette = Palette::new(cx, items.values().cloned().collect(), matcher, on_accept);

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

enum CommandEvent {
    NewCommand((&'static str, i64), Arc<Command>),
    RemoveCommand((&'static str, i64)),
}

impl EventEmitter<CommandEvent> for CommandPalette {}

pub trait CommandManager {
    fn register_command(&mut self, name: (&'static str, i64), command: Arc<Command>);
    fn unregister_command(&mut self, name: (&'static str, i64));
}

impl CommandManager for App {
    fn register_command(&mut self, name: (&'static str, i64), command: Arc<Command>) {
        let commands = self.global::<CommandPaletteHolder>().0.clone();
        commands.update(self, move |_, cx| {
            cx.emit(CommandEvent::NewCommand(name, command));
        })
    }

    fn unregister_command(&mut self, name: (&'static str, i64)) {
        let commands = self.global::<CommandPaletteHolder>().0.clone();
        commands.update(self, move |_, cx| {
            cx.emit(CommandEvent::RemoveCommand(name));
        })
    }
}

pub struct CommandPaletteHolder(Entity<CommandPalette>);

impl CommandPaletteHolder {
    pub fn new(palette: Entity<CommandPalette>) -> Self {
        Self(palette)
    }
}

impl Global for CommandPaletteHolder {}
