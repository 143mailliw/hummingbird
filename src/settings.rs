pub mod playback;
pub mod scan;
pub mod storage;

use std::{fs::File, path::PathBuf, sync::mpsc::channel, time::Duration};

use gpui::{App, AppContext, AsyncApp, Entity, Global};
use notify::{Event, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default)]
    pub scanning: scan::ScanSettings,
    #[serde(default)]
    pub playback: playback::PlaybackSettings,
}

pub fn create_settings(path: &PathBuf) -> Settings {
    let Ok(file) = File::open(path) else {
        return Settings::default();
    };
    let reader = std::io::BufReader::new(file);

    if let Ok(settings) = serde_json::from_reader(reader) {
        settings
    } else {
        warn!("Failed to parse settings file, using default settings");
        Settings::default()
    }
}

pub struct SettingsGlobal {
    pub model: Entity<Settings>,
    #[allow(dead_code)]
    pub watcher: Option<Box<dyn Watcher>>,
}

impl Global for SettingsGlobal {}

pub fn setup_settings(cx: &mut App, path: PathBuf) {
    let settings = cx.new(|_| create_settings(&path));
    let settings_model = settings.clone(); // for the closure

    // create and setup file watcher
    let (tx, rx) = channel::<notify::Result<Event>>();

    let watcher = notify::recommended_watcher(tx);

    let Ok(mut watcher) = watcher else {
        warn!("failed to create settings watcher");

        let global = SettingsGlobal {
            model: settings,
            watcher: None,
        };

        cx.set_global(global);
        return;
    };
    if let Err(e) = watcher.watch(path.parent().unwrap(), RecursiveMode::Recursive) {
        warn!("failed to watch settings file: {:?}", e);
    }

    cx.spawn(async move |app: &mut AsyncApp| {
        loop {
            while let Ok(event) = rx.try_recv() {
                match event {
                    Ok(v) => {
                        if !v.paths.iter().any(|t| t.ends_with("settings.json")) {
                            return;
                        };
                        match v.kind {
                            notify::EventKind::Create(_) | notify::EventKind::Modify(_) => {
                                info!("Settings changed, updating...");
                                let settings = create_settings(&path);
                                settings_model
                                    .update(app, |v, _| {
                                        *v = settings;
                                    })
                                    .expect("settings model could not be updated");
                            }
                            notify::EventKind::Remove(_) => {
                                info!("Settings file removed, using default settings");
                                settings_model
                                    .update(app, |v, _| {
                                        *v = Settings::default();
                                    })
                                    .expect("settings model could not be updated");
                            }
                            _ => (),
                        }
                    }
                    Err(e) => warn!("watch error: {:?}", e),
                }
            }

            app.background_executor()
                .timer(Duration::from_millis(10))
                .await;
        }
    })
    .detach();

    let global = SettingsGlobal {
        model: settings,
        watcher: Some(Box::new(watcher)),
    };

    cx.set_global(global);
}
