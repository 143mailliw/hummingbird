// On Windows do NOT show a console window when opening the app
#![cfg_attr(
    all(not(test), not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::sync::LazyLock;

use services::mmb::lastfm::{LASTFM_API_KEY, LASTFM_API_SECRET};

mod devices;
mod library;
mod media;
mod playback;
mod services;
mod settings;
mod ui;
mod util;

static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap()
});

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting application");

    if LASTFM_API_KEY.is_none() || LASTFM_API_SECRET.is_none() {
        tracing::warn!(
            "Binary not compiled with LastFM support, set LASTFM_API_KEY and LASTFM_API_SECRET at compile time to enable"
        );
    }

    crate::ui::app::run()
}
