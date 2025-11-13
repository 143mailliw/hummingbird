use gpui::{App, AppContext};
use sqlx::SqlitePool;
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::error;

use crate::ui::app::Pool;

#[cfg(windows)]
const LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &str = "\n";

#[derive(sqlx::FromRow)]
struct PlaylistEntry {
    location: String,
    duration: i64,
    track_artist_names: String,
    artist_name: String,
    track_title: String,
    album_title: String,
}

async fn make_m3u(pool: &SqlitePool, pl_id: i64) -> anyhow::Result<String> {
    let mut output = String::new();

    output.push_str(&format!("#EXTM3U{LINE_ENDING}"));

    let query = include_str!("../../queries/playlist/list_tracks_for_export.sql");
    let data: Vec<PlaylistEntry> = sqlx::query_as(query).bind(pl_id).fetch_all(pool).await?;

    data.iter().for_each(|entry| {
        output.push_str(&format!(
            "#EXTINF:{},{} - {}{LINE_ENDING}",
            entry.duration, entry.track_artist_names, entry.track_title
        ));
        output.push_str(&format!("#EXTALB:{}{LINE_ENDING}", entry.album_title));
        output.push_str(&format!("#EXTART:{}{LINE_ENDING}", entry.artist_name));
        output.push_str(&format!("{}{LINE_ENDING}", entry.location));
        output.push_str(LINE_ENDING);
    });

    Ok(output)
}

pub fn export_playlist(cx: &mut App, pl_id: i64, playlist_name: &str) -> anyhow::Result<()> {
    let dirs = directories::UserDirs::new()
        .ok_or_else(|| anyhow::anyhow!("Failed to get user directory"))?;
    let dir = dirs
        .document_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get documents directory"))?;

    let suggested_name = format!("{playlist_name}.m3u");

    let path_future = cx.prompt_for_new_path(dir, Some(&suggested_name));

    let pool = cx.global::<Pool>().0.clone();

    crate::RUNTIME.spawn(async move {
        let result = async {
            let path = path_future.await??;

            if let Some(path) = path {
                let output = make_m3u(&pool, pl_id).await?;
                let mut file = File::create(path).await?;
                file.write_all(output.as_bytes()).await?;
            }

            anyhow::Ok(())
        }
        .await;

        if let Err(err) = result {
            error!("Failed to export playlist: {err}");
        }
    });

    Ok(())
}
