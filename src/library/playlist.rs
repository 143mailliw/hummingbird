use std::{ffi::OsStr, path::PathBuf};

use futures::future::join_all;
use gpui::{App, PathPromptOptions};
use sqlx::{Sqlite, SqlitePool};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncWriteExt},
};
use tracing::error;

use crate::ui::{
    app::Pool,
    models::{Models, PlaylistEvent},
};

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

    let suggested_name = format!("{playlist_name}.m3u8");

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

struct M3UEntry {
    duration: Option<u32>,
    track_artist_names: Option<String>,
    track_title: Option<String>,
    album_title: Option<String>,
    artist_name: Option<String>,
    location: PathBuf,
}

async fn parse_m3u(path: &PathBuf) -> anyhow::Result<Vec<M3UEntry>> {
    let file = File::open(path).await?;
    let reader = tokio::io::BufReader::new(file);
    let mut lines = reader.lines();

    let mut entries = Vec::new();
    let mut current_entry = M3UEntry {
        duration: None,
        track_artist_names: None,
        track_title: None,
        album_title: None,
        artist_name: None,
        location: PathBuf::new(),
    };

    while let Some(line) = lines.next_line().await? {
        if let Some(line) = line.strip_prefix("#EXTINF:") {
            let info: Vec<&str> = line.splitn(2, ',').collect();

            if info.len() == 2 {
                current_entry.duration = info[0].parse::<u32>().ok();

                let delims = ['-', ':', '–'];
                let title_artist: Vec<&str> = info[1].splitn(2, &delims).collect();
                if title_artist.len() == 2 {
                    current_entry.track_artist_names = Some(title_artist[0].trim().to_string());
                    current_entry.track_title = Some(title_artist[1].trim().to_string());
                } else {
                    current_entry.track_title = Some(info[1].to_string());
                }
            }
        } else if let Some(album_title) = line.strip_prefix("#EXTALB:") {
            current_entry.album_title = Some(album_title.to_string());
        } else if let Some(artist_name) = line.strip_prefix("#EXTART:") {
            current_entry.artist_name = Some(artist_name.to_string());
        } else if !line.starts_with('#') && !line.is_empty() {
            current_entry.location = line.into();
            entries.push(current_entry);
            current_entry = M3UEntry {
                duration: None,
                track_artist_names: None,
                track_title: None,
                album_title: None,
                artist_name: None,
                location: PathBuf::new(),
            };
        }
    }

    Ok(entries)
}

pub fn import_playlist(cx: &mut App, playlist_id: i64) -> anyhow::Result<()> {
    let path_future = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Select a M3U file...".into()),
    });

    let pool = cx.global::<Pool>().0.clone();
    let playlist_tracker = cx.global::<Models>().playlist_tracker.clone();

    cx.spawn(async move |cx| {
        crate::RUNTIME
            .spawn(async move {
                let result = async {
                    let path = path_future.await??;

                    if let Some(path) = path.as_ref().and_then(|v| v.first()) {
                        let data = parse_m3u(path).await?;

                        let lookup_query = include_str!("../../queries/playlist/lookup_track.sql");
                        let iter = data.into_iter().map(|entry| {
                            sqlx::query_scalar::<Sqlite, i64>(lookup_query)
                                .bind(entry.location.to_string_lossy().to_string())
                                .bind(entry.track_title)
                                .bind(entry.artist_name)
                                .bind(entry.album_title)
                                .bind(entry.track_artist_names)
                                .bind(entry.duration)
                                .bind(format!(
                                    "%{}%",
                                    entry
                                        .location
                                        .file_prefix()
                                        .map(OsStr::to_str)
                                        .flatten()
                                        .unwrap_or_default()
                                ))
                                .fetch_one(&pool)
                        });

                        let ids = join_all(iter).await.into_iter().flatten();

                        let mut tx = pool.begin().await?;

                        let reset_query = include_str!("../../queries/playlist/empty_playlist.sql");
                        sqlx::query(reset_query)
                            .bind(playlist_id)
                            .execute(&mut *tx)
                            .await?;

                        let insert_query = include_str!("../../queries/playlist/add_track.sql");

                        for track_id in ids {
                            sqlx::query(insert_query)
                                .bind(playlist_id)
                                .bind(track_id)
                                .execute(&mut *tx)
                                .await?;
                        }

                        tx.commit().await?;
                    }

                    anyhow::Ok(())
                }
                .await;

                if let Err(err) = result {
                    error!("Failed to import playlist: {err}");
                }
            })
            .await
            .ok();

        playlist_tracker.update(cx, |_, cx| {
            cx.emit(PlaylistEvent::PlaylistUpdated(playlist_id))
        })
    })
    .detach();

    Ok(())
}
