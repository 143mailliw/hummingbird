use gpui::{App, KeyBinding, Menu, MenuItem, SharedString, actions};
use tracing::{debug, info};

use crate::{
    library::scan::ScanInterface,
    playback::{interface::PlaybackInterface, thread::PlaybackState},
};

use super::models::{Models, PlaybackInfo};

actions!(
    hummingbird,
    [About, Quit, PlayPause, Next, Previous, Search, ForceScan]
);

actions!(hummingbird, [HideSelf, HideOthers, ShowAll]);

pub fn register_actions(cx: &mut App) {
    debug!("registering actions");
    cx.on_action(quit);
    cx.on_action(play_pause);
    cx.on_action(next);
    cx.on_action(previous);
    cx.on_action(hide_self);
    cx.on_action(hide_others);
    cx.on_action(show_all);
    cx.on_action(about);
    cx.on_action(force_scan);
    debug!("actions: {:?}", cx.all_action_names());
    debug!("action available: {:?}", cx.is_action_available(&Quit));
    if cfg!(target_os = "macos") {
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.bind_keys([KeyBinding::new("cmd-right", Next, None)]);
        cx.bind_keys([KeyBinding::new("cmd-left", Previous, None)]);
        cx.bind_keys([KeyBinding::new("cmd-f", Search, None)]);
        cx.bind_keys([KeyBinding::new("cmd-h", HideSelf, None)]);
        cx.bind_keys([KeyBinding::new("cmd-alt-h", HideOthers, None)]);
    } else {
        cx.bind_keys([KeyBinding::new("ctrl-w", Quit, None)]);
        cx.bind_keys([KeyBinding::new("ctrl-right", Next, None)]);
        cx.bind_keys([KeyBinding::new("ctrl-left", Previous, None)]);
        cx.bind_keys([KeyBinding::new("ctrl-f", Search, None)]);
    }
    cx.bind_keys([KeyBinding::new("alt-shift-s", ForceScan, None)]);
    cx.bind_keys([KeyBinding::new("space", PlayPause, None)]);
    cx.set_menus(vec![
        Menu {
            name: SharedString::from("Hummingbird"),
            items: vec![
                MenuItem::action("About Hummingbird", About),
                MenuItem::separator(),
                MenuItem::submenu(Menu {
                    name: SharedString::from("Services"),
                    items: vec![],
                }),
                MenuItem::separator(),
                MenuItem::action("Hide Hummingbird", HideSelf),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quit Hummingbird", Quit),
            ],
        },
        Menu {
            name: SharedString::from("View"),
            items: vec![],
        },
        Menu {
            name: SharedString::from("Window"),
            items: vec![],
        },
    ]);
}

fn quit(_: &Quit, cx: &mut App) {
    info!("Quitting...");
    cx.quit();
}

fn play_pause(_: &PlayPause, cx: &mut App) {
    let state = cx.global::<PlaybackInfo>().playback_state.read(cx);
    let interface = cx.global::<PlaybackInterface>();
    match state {
        PlaybackState::Stopped => {
            interface.play();
        }
        PlaybackState::Playing => {
            interface.pause();
        }
        PlaybackState::Paused => {
            interface.play();
        }
    }
}

fn next(_: &Next, cx: &mut App) {
    let interface = cx.global::<PlaybackInterface>();
    interface.next();
}

fn previous(_: &Previous, cx: &mut App) {
    let interface = cx.global::<PlaybackInterface>();
    interface.previous();
}

fn hide_self(_: &HideSelf, cx: &mut App) {
    cx.hide();
}

fn hide_others(_: &HideOthers, cx: &mut App) {
    cx.hide_other_apps();
}

fn show_all(_: &ShowAll, cx: &mut App) {
    cx.unhide_other_apps();
}

fn about(_: &About, cx: &mut App) {
    let show_about = cx.global::<Models>().show_about.clone();
    show_about.write(cx, true);
}

fn force_scan(_: &ForceScan, cx: &mut App) {
    let scanner = cx.global::<ScanInterface>();
    scanner.force_scan();
}
