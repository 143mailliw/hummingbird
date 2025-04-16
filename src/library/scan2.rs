// `scan2` is/will be a work-in-progress asynchronous thread-safe library scanner designed to
// eliminate I/O bottlenecks while reducing per-thread memory usage by taking advantage of GPUI's
// built-in async runtime.
//
// Eventually, this implementation will be made available through an environment variable once it
// lands in master, then (once it is proven functional), replace the `scan` module completely.
//
// !> In order for `scan2` to be thread safe, the SQLite database must be opened in "serialized"
// !> mode (see https://www.sqlite.org/threadsafe.html) - the implementation will assume that
// !> queries are executed in seqeuential order in order to reduce synchronization complexity.
//
// `scan2` will also solve the data deduplication problem by locking albums behind folder/disc
// pairs, and optionally using MBIDs if available.

use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use async_fs::{metadata, read, read_dir};
use async_lock::Mutex;
use dashmap::DashMap;
use fnv::{FnvBuildHasher, FnvHasher};
use futures::future::join_all;
use futures::join;
use globwalk::GlobWalkerBuilder;
use gpui::{App, AppContext};
use image::codecs::jpeg::JpegEncoder;
use image::imageops::thumbnail;
use image::{EncodableLayout, ImageReader};
use smol::stream::StreamExt;
use sqlx::SqlitePool;
use tracing::{error, info_span, span, warn, Instrument, Level};

use crate::media::builtin::symphonia::SymphoniaProvider;
use crate::media::metadata::Metadata;
use crate::media::traits::{MediaPlugin, MediaProvider};
use crate::settings::{Settings, SettingsGlobal};
use crate::ui::app::Pool;

struct FileInformation {
    metadata: Metadata,
    duration: u64,
    album_art: Option<Box<[u8]>>,
}

type ScanRecord = DashMap<PathBuf, u64, FnvBuildHasher>;
type MutexTable<T: std::hash::Hash> = Arc<Mutex<HashMap<T, Option<i64>, FnvBuildHasher>>>;

#[derive(Debug, Clone)]
struct ScanState {
    scan_record: ScanRecord,
    pool: SqlitePool,
    scan_record_path: std::path::PathBuf,
    artist_mutex_table: MutexTable<String>,
    album_mutex_table: MutexTable<(i64, String)>,
}

async fn scan(scan_state: Arc<ScanState>, path: PathBuf) -> anyhow::Result<()> {
    let mut dir_futures: Vec<_> = Vec::new();
    let mut track_futures: Vec<_> = Vec::new();

    if path.exists() {
        let mut children = read_dir(&path).await?;

        while let Some(child) = children.next().await {
            let considered_path = child?.path();

            if considered_path.is_dir() {
                let scan = scan(scan_state.clone(), considered_path);

                dir_futures.push(Box::pin(scan));
            } else {
                let timestamp = metadata(&considered_path)
                    .await?
                    .modified()?
                    .duration_since(SystemTime::UNIX_EPOCH)?
                    .as_secs();

                if let Some(time) = scan_state.scan_record.get(&considered_path) {
                    if *time == timestamp {
                        continue;
                    }
                }

                scan_state
                    .scan_record
                    .insert(considered_path.clone(), timestamp);

                // SqlitePool is in a Arc already, no reason to provide the entire scan_state
                let scan = scan_track(considered_path, scan_state.clone());

                track_futures.push(Box::pin(scan));
            }
        }
    }

    let results = join!(join_all(dir_futures), join_all(track_futures));

    for result in results.0 {
        if let anyhow::Result::Err(e) = result {
            warn!(
                "Error occured during scanning subdirectory in directory {:?}: {}",
                path, e
            );
        }
    }

    for result in results.1 {
        if let anyhow::Result::Err(e) = result {
            warn!(
                "Error occured during scanning track in directory {:?}: {}",
                path, e
            );
        }
    }

    Ok(())
}

/// Returns the first image (cover/front/folder.jpeg/png/jpeg) in the track's containing folder
/// Album art can be named anything, but this pattern is convention and the least likely to return a false positive
async fn scan_path_for_album_art(path: &Path) -> anyhow::Result<Option<Box<[u8]>>> {
    // no async equivalent
    let glob = GlobWalkerBuilder::from_patterns(
        path.parent().unwrap(),
        &["{folder,cover,front}.{jpg,jpeg,png}"],
    )
    .case_insensitive(true)
    .max_depth(1)
    .build()?
    .filter_map(|e| e.ok());

    for entry in glob {
        if let Ok(bytes) = read(entry.path()).await {
            return Ok(Some(bytes.into_boxed_slice()));
        }
    }

    Ok(None)
}

async fn save_scan_record(scan_state: &ScanState) -> anyhow::Result<()> {
    let scan_map: Vec<(PathBuf, u64)> = scan_state
        .scan_record
        .iter()
        .map(|entry| (entry.key().clone(), *entry.value()))
        .collect();

    let json = serde_json::to_string(&scan_map)?;

    async_fs::write(&scan_state.scan_record_path, json).await?;

    Ok(())
}

async fn read_metadata_for_path(path: &Path) -> anyhow::Result<FileInformation> {
    // TODO: use provider table
    // This is blocked by MediaProvider, which needs to be replaced with a factory of audio streams
    let mut provider = SymphoniaProvider::default();
    let src = std::fs::File::open(path)?;

    provider.open(src, None)?;
    provider.start_playback()?;

    let metadata = provider.read_metadata().cloned()?;
    let album_art = provider.read_image()?;
    let duration = provider.duration_secs()?;

    provider.close()?;

    Ok(FileInformation {
        metadata,
        album_art: if album_art.is_some() {
            album_art
        } else {
            scan_path_for_album_art(path).await?
        },
        duration,
    })
}

async fn insert_artist(
    metadata: &Metadata,
    pool: &SqlitePool,
    artist_mutex_table: &MutexTable<String>,
) -> anyhow::Result<Option<i64>> {
    let Some(artist) = metadata.album_artist.as_ref().or(metadata.artist.as_ref()) else {
        return Ok(None);
    };

    let mut lock = artist_mutex_table.lock().await;

    if let Some(id) = lock.get(artist).cloned().flatten() {
        return Ok(Some(id));
    }

    let found: Result<(i64,), sqlx::Error> =
        sqlx::query_as(include_str!("../../queries/scan/get_artist_id.sql"))
            .bind(artist)
            .fetch_one(pool)
            .await;

    match found {
        Ok(v) => {
            lock.insert(artist.clone(), Some(v.0));
            Ok(Some(v.0))
        }
        Err(sqlx::Error::RowNotFound) => {
            let insert: (i64,) =
                sqlx::query_as(include_str!("../../queries/scan/create_artist.sql"))
                    .bind(artist)
                    .bind(metadata.artist_sort.as_ref().unwrap_or(artist))
                    .fetch_one(pool)
                    .await?;

            lock.insert(artist.clone(), Some(insert.0));
            Ok(Some(insert.0))
        }
        Err(e) => Err(e)?,
    }
}

async fn insert_album(
    metadata: &Metadata,
    artist_id: i64,
    image: &Option<Box<[u8]>>,
    pool: &SqlitePool,
    album_mutex_table: &MutexTable<(i64, String)>,
) -> anyhow::Result<Option<i64>> {
    let Some(album) = &metadata.album else {
        return Ok(None);
    };

    let mut lock = album_mutex_table.lock().await;

    if let Some(id) = lock.get(&(artist_id, album.clone())).cloned().flatten() {
        return Ok(Some(id));
    }

    let result: Result<(i64,), sqlx::Error> =
        sqlx::query_as(include_str!("../../queries/scan/get_album_id.sql"))
            .bind(album)
            .fetch_one(pool)
            .await;

    match result {
        Ok(v) => {
            let id = v.0;
            lock.insert((artist_id, album.clone()), Some(id));
            Ok(Some(id))
        }
        Err(sqlx::Error::RowNotFound) => {
            let images: Option<anyhow::Result<(Vec<u8>, Vec<u8>)>> = image.as_ref().map(|v| {
                let reader = Cursor::new(v);
                let decoded = ImageReader::new(reader)
                    .with_guessed_format()?
                    .decode()?
                    .into_rgba8();

                // make bitmap thumbnail to speed up loading of library lists
                let thumb = thumbnail(&decoded, 70, 70);

                let mut thumb_buf: Cursor<Vec<u8>> = Cursor::new(Vec::new());

                thumb.write_to(&mut thumb_buf, image::ImageFormat::Bmp)?;
                thumb_buf.flush()?;

                // downscale the image to 1024x1024 if it's too large
                let size = decoded.dimensions();
                // let downscaled = if size.0 <= 1024 || size.1 <= 1024 {
                //     v.clone().to_vec()
                // } else {
                //     let resized = image::imageops::resize(
                //         &decoded,
                //         1024,
                //         1024,
                //         image::imageops::FilterType::Lanczos3,
                //     );
                //     let mut buf: Cursor<Vec<u8>> = Cursor::new(Vec::new());
                //     let mut encoder = JpegEncoder::new_with_quality(&mut buf, 70);

                //     encoder.encode(
                //         resized.as_bytes(),
                //         resized.width(),
                //         resized.height(),
                //         image::ExtendedColorType::Rgb8,
                //     )?;
                //     buf.flush()?;

                //     buf.into_inner()
                // };
                let downscaled = v.clone().to_vec();

                anyhow::Result::Ok((thumb_buf.into_inner(), downscaled))
            });

            // handle errors and break inner tuple
            let (thumb, full) = if let Some(anyhow::Result::Err(e)) = images.as_ref() {
                warn!(
                    "Error processing image for album\"{}\": {}",
                    metadata.album.as_ref().unwrap(),
                    e
                );
                warn!("The album will still be added to your library, but it won't have any art.");
                (None, None)
            } else if let Some(anyhow::Result::Ok((thumb, full))) = images {
                (Some(thumb), Some(full))
            } else {
                (None, None)
            };

            let insert: (i64,) =
                sqlx::query_as(include_str!("../../queries/scan/create_album.sql"))
                    .bind(album.clone())
                    .bind(metadata.sort_album.as_ref().unwrap_or(album))
                    .bind(artist_id)
                    .bind(full)
                    .bind(thumb)
                    .bind(metadata.date)
                    .bind(&metadata.label)
                    .bind(&metadata.catalog)
                    .bind(&metadata.isrc)
                    .fetch_one(pool)
                    .await?;

            let id = insert.0;
            lock.insert((artist_id, album.clone()), Some(id));

            Ok(Some(insert.0))
        }
        Err(e) => Err(e)?,
    }
}

async fn insert_track(
    metadata: &Metadata,
    album_id: i64,
    path: &Path,
    duration: u64,
    pool: &SqlitePool,
) -> anyhow::Result<()> {
    let name = metadata
        .name
        .clone()
        .or_else(|| {
            path.file_name()
                .and_then(|x| x.to_str())
                .map(|x| x.to_string())
        })
        .ok_or_else(|| anyhow::anyhow!("couldn't find or construct track name"))?;

    let _: (i64,) = sqlx::query_as(include_str!("../../queries/scan/create_track.sql"))
        .bind(&name)
        .bind(&name)
        .bind(album_id)
        .bind(metadata.track_current.map(|x| x as i32))
        .bind(metadata.disc_current.map(|x| x as i32))
        .bind(duration as i32)
        .bind(path.to_str())
        .bind(&metadata.genre)
        .fetch_one(pool)
        .await?;

    Ok(())
}

fn file_is_scannable(path: &Path, exts: &&[&str]) -> bool {
    for extension in exts.iter() {
        if let Some(ext) = path.extension() {
            if ext == *extension {
                return true;
            }
        }
    }

    false
}

async fn scan_track(path: PathBuf, scan_state: Arc<ScanState>) -> anyhow::Result<()> {
    if !file_is_scannable(path.as_ref(), &SymphoniaProvider::SUPPORTED_EXTENSIONS) {
        return Ok(());
    }

    let info = read_metadata_for_path(path.as_ref()).await?;

    let Some(artist_id) = insert_artist(
        &info.metadata,
        &scan_state.pool,
        &scan_state.artist_mutex_table,
    )
    .await?
    else {
        return Ok(());
    };

    let Some(album_id) = insert_album(
        &info.metadata,
        artist_id,
        &info.album_art,
        &scan_state.pool,
        &scan_state.album_mutex_table,
    )
    .await?
    else {
        return Ok(());
    };

    insert_track(
        &info.metadata,
        album_id,
        path.as_ref(),
        info.duration,
        &scan_state.pool,
    )
    .await?;

    Ok(())
}

pub fn start_scan(cx: &mut App) {
    let paths = cx
        .global::<SettingsGlobal>()
        .model
        .read(cx)
        .scanning
        .paths
        .clone();
    let pool = cx.global::<Pool>().0.clone();

    let dirs = directories::ProjectDirs::from("me", "william341", "muzak")
        .expect("couldn't find project dirs");
    let directory = dirs.data_dir();
    if !directory.exists() {
        std::fs::create_dir(directory).expect("couldn't create data directory");
    }
    let record_path = directory.join("scan_record.json");

    let record = if record_path.exists() {
        let file = std::fs::File::open(&record_path);

        let Ok(file) = file else {
            return;
        };
        let reader = std::io::BufReader::new(file);

        match serde_json::from_reader(reader) {
            Ok(scan_record) => scan_record,
            Err(e) => {
                error!("could not read scan record: {:?}", e);
                error!("scanning will be slow until the scan record is rebuilt");
                DashMap::with_hasher(FnvBuildHasher::new())
            }
        }
    } else {
        DashMap::with_hasher(FnvBuildHasher::new())
    };

    cx.background_spawn(async move {
        let scan_state = Arc::new(ScanState {
            scan_record: record,
            pool,
            scan_record_path: record_path,
            artist_mutex_table: Arc::new(Mutex::new(HashMap::with_hasher(FnvBuildHasher::new()))),
            album_mutex_table: Arc::new(Mutex::new(HashMap::with_hasher(FnvBuildHasher::new()))),
        });

        let mut futures_vec: Vec<_> = Vec::new();

        for path in paths {
            futures_vec.push(Box::pin(scan(scan_state.clone(), path)));
        }

        join_all(futures_vec).instrument(info_span!("scan2")).await;

        if let Err(e) = save_scan_record(scan_state.as_ref()).await {
            warn!("Failed to save scan record: {}", e);
            warn!("Scan will completely restart when you next start");
        }
    })
    .detach();
}
