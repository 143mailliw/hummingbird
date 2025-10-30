use std::{fs::File, io::BufReader, path::PathBuf, sync::mpsc::channel, time::Duration};

use gpui::{App, AppContext, AsyncApp, EventEmitter, Global, Rgba, rgb, rgba};
use notify::{Event, RecursiveMode, Watcher};
use serde::Deserialize;
use tracing::{error, info, warn};

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Theme {
    pub background_primary: Rgba,
    pub background_secondary: Rgba,
    pub background_tertiary: Rgba,

    pub border_color: Rgba,

    pub album_art_background: Rgba,

    pub text: Rgba,
    pub text_secondary: Rgba,
    pub text_link: Rgba,

    pub nav_button_hover: Rgba,
    pub nav_button_active: Rgba,

    pub playback_button: Rgba,
    pub playback_button_hover: Rgba,
    pub playback_button_active: Rgba,
    pub playback_button_border: Rgba,
    pub playback_button_toggled: Rgba,

    pub window_button: Rgba,
    pub window_button_hover: Rgba,
    pub window_button_active: Rgba,

    pub close_button: Rgba,
    pub close_button_hover: Rgba,
    pub close_button_active: Rgba,

    pub queue_item: Rgba,
    pub queue_item_hover: Rgba,
    pub queue_item_active: Rgba,
    pub queue_item_current: Rgba,

    pub button_primary: Rgba,
    pub button_primary_hover: Rgba,
    pub button_primary_active: Rgba,
    pub button_primary_text: Rgba,

    pub button_secondary: Rgba,
    pub button_secondary_hover: Rgba,
    pub button_secondary_active: Rgba,
    pub button_secondary_text: Rgba,

    pub button_warning: Rgba,
    pub button_warning_hover: Rgba,
    pub button_warning_active: Rgba,
    pub button_warning_text: Rgba,

    pub button_danger: Rgba,
    pub button_danger_hover: Rgba,
    pub button_danger_active: Rgba,
    pub button_danger_text: Rgba,

    pub slider_foreground: Rgba,
    pub slider_background: Rgba,

    pub elevated_background: Rgba,
    pub elevated_border_color: Rgba,

    pub menu_item: Rgba,
    pub menu_item_hover: Rgba,
    pub menu_item_active: Rgba,

    pub modal_overlay_bg: Rgba,

    pub text_input_selection: Rgba,
    pub caret_color: Rgba,

    pub palette_item_hover: Rgba,
    pub palette_item_active: Rgba,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            background_primary: rgb(0x0C1116),
            background_secondary: rgb(0x161A22),
            background_tertiary: rgb(0x222831),

            border_color: rgb(0x272D37),

            album_art_background: rgb(0x4C5974),

            text: rgb(0xF4F5F6),
            text_secondary: rgb(0xBEC4CA),
            text_link: rgb(0x5279D4),

            nav_button_hover: rgb(0x161A22),
            nav_button_active: rgb(0x0A0E12),

            playback_button: rgba(0x282F3D00),
            playback_button_hover: rgb(0x282F3D),
            playback_button_active: rgb(0x0D1014),
            playback_button_border: rgba(0x37404E00),
            playback_button_toggled: rgb(0x0667B2),

            window_button: rgba(0x33415500),
            window_button_hover: rgb(0x282F3D),
            window_button_active: rgb(0x0D1014),

            queue_item: rgb(0x161A2200),
            queue_item_hover: rgb(0x161A22),
            queue_item_active: rgb(0x0C1116),
            queue_item_current: rgb(0x272D37),

            close_button: rgba(0x282F3D00),
            close_button_hover: rgb(0xAE0909),
            close_button_active: rgb(0x7A0606),

            button_primary: rgb(0x0667B2),
            button_primary_hover: rgb(0x087AD1),
            button_primary_active: rgb(0x065D9F),
            button_primary_text: rgb(0xE0F1FE),

            button_secondary: rgb(0x37404E),
            button_secondary_hover: rgb(0x495467),
            button_secondary_active: rgb(0x262C36),
            button_secondary_text: rgb(0xBEC4CA),

            button_warning: rgb(0xEDB407),
            button_warning_hover: rgb(0xF8C017),
            button_warning_active: rgb(0xD6A207),
            button_warning_text: rgb(0xFEF8E5),

            button_danger: rgb(0xCD0B0B),
            button_danger_hover: rgb(0xE80C0C),
            button_danger_active: rgb(0xB70A0A),
            button_danger_text: rgb(0xFEE3E3),

            slider_foreground: rgb(0x0673C6),
            slider_background: rgb(0x37404E),

            elevated_background: rgb(0x161A22),
            elevated_border_color: rgb(0x272D37),

            menu_item: rgba(0x282F3D00),
            menu_item_hover: rgb(0x282F3D),
            menu_item_active: rgb(0x0D1014),

            modal_overlay_bg: rgba(0x0C111655),

            text_input_selection: rgba(0x0673C688),
            caret_color: rgb(0xF4F5F6),

            palette_item_hover: rgb(0x282F3D),
            palette_item_active: rgb(0x0D1014),
        }
    }
}

impl Global for Theme {}

pub fn create_theme(path: &PathBuf) -> Theme {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);

        if let Ok(theme) = serde_json::from_reader(reader) {
            theme
        } else {
            warn!("Theme file exists but it could not be loaded, using default");
            Theme::default()
        }
    } else {
        Theme::default()
    }
}

#[derive(PartialEq, Clone)]
pub struct ThemeEvTransmitter;

impl EventEmitter<Theme> for ThemeEvTransmitter {}

#[allow(dead_code)]
pub struct ThemeWatcher(pub Box<dyn Watcher>);

impl Global for ThemeWatcher {}

pub fn setup_theme(cx: &mut App, path: PathBuf) {
    cx.set_global(create_theme(&path));
    let theme_transmitter = cx.new(|_| ThemeEvTransmitter);

    cx.subscribe(&theme_transmitter, |_, theme, cx| {
        cx.set_global(theme.clone());
        cx.refresh_windows();
    })
    .detach();

    let (tx, rx) = channel::<notify::Result<Event>>();

    let watcher = notify::recommended_watcher(tx);

    if let Ok(mut watcher) = watcher {
        if let Err(e) = watcher.watch(path.parent().unwrap(), RecursiveMode::NonRecursive) {
            warn!("failed to watch settings directory: {:?}", e);
        }

        cx.spawn(async move |cx: &mut AsyncApp| {
            loop {
                while let Ok(event) = rx.try_recv() {
                    match event {
                        Ok(v) => {
                            if v.paths.iter().any(|t| t.ends_with("theme.json")) {
                                match v.kind {
                                    notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                                        info!("Theme changed, updating...");
                                        let theme = create_theme(&path);
                                        theme_transmitter
                                            .update(cx, move |_, m| {
                                                m.emit(theme);
                                            })
                                            .expect("could not send theme to main thread");
                                    }
                                    notify::EventKind::Remove(_) => {
                                        info!("Theme file removed, resetting to default...");
                                        theme_transmitter
                                            .update(cx, |_, m| {
                                                m.emit(Theme::default());
                                            })
                                            .expect("could not send theme to main thread");
                                    }
                                    _ => (),
                                }
                            }
                        }
                        Err(e) => error!("error occured while watching theme.json: {:?}", e),
                    }
                }

                cx.background_executor()
                    .timer(Duration::from_millis(10))
                    .await;
            }
        })
        .detach();

        // store the watcher in a global so it doesn't go out of scope
        let tw = ThemeWatcher(Box::new(watcher));
        cx.set_global(tw);
    } else if let Err(e) = watcher {
        warn!("failed to watch settings directory: {:?}", e);
    }
}
