// On Windows do NOT show a console window when opening the app
#![cfg_attr(
    all(not(test), not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use services::mmb::lastfm::{LASTFM_API_KEY, LASTFM_API_SECRET};
use smol_macros::main;

mod devices;
mod library;
mod media;
mod playback;
mod services;
mod settings;
mod ui;
mod util;

main! {
    async fn main() {
        tracing_subscriber::fmt::init();

        tracing::info!("Starting application");

        if LASTFM_API_KEY.is_none() || LASTFM_API_SECRET.is_none() {
            tracing::warn!("Binary not compiled with LastFM support, set LASTFM_API_KEY and LASTFM_API_SECRET at compile time to enable");
        }

        crate::ui::app::run().await;
    }
}
