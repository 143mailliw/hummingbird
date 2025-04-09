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

use std::io::{Cursor, Write};
use std::path::Path;

use async_std::fs::{read, read_dir};
use async_std::path::PathBuf;
use async_std::stream::StreamExt;
use futures::future::join_all;
use futures::join;
use globwalk::GlobWalkerBuilder;
use image::codecs::jpeg::JpegEncoder;
use image::imageops::thumbnail;
use image::{EncodableLayout, ImageReader};
use sqlx::SqlitePool;
use tracing::warn;

use crate::media::builtin::symphonia::SymphoniaProvider;
use crate::media::metadata::Metadata;
use crate::media::traits::{MediaPlugin, MediaProvider};

struct FileInformation {
    metadata: Metadata,
    duration: u64,
    album_art: Option<Box<[u8]>>,
}

// SqlitePool is alerady an Arc under the hood, no point in wrapping it
async fn scan(path: PathBuf, pool: SqlitePool) -> anyhow::Result<()> {
    let mut dir_futures: Vec<_> = Vec::new();
    let mut track_futures: Vec<_> = Vec::new();

    if path.exists().await {
        let mut children = read_dir(&path).await?;

        while let Some(child) = children.next().await {
            let considered_path = child?.path();

            if considered_path.is_dir().await {
                let scan = scan(considered_path, pool.clone());

                dir_futures.push(Box::pin(scan));
            } else {
                let scan = scan_track(considered_path, pool.clone());

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

async fn insert_artist(metadata: &Metadata, pool: &SqlitePool) -> anyhow::Result<Option<i64>> {
    let Some(artist) = metadata.album_artist.as_ref().or(metadata.artist.as_ref()) else {
        return Ok(None);
    };

    let found: Result<(i64,), sqlx::Error> =
        sqlx::query_as(include_str!("../../queries/scan/get_artist_id.sql"))
            .bind(artist)
            .fetch_one(pool)
            .await;

    match found {
        Ok(v) => Ok(Some(v.0)),
        Err(sqlx::Error::RowNotFound) => {
            let insert: (i64,) =
                sqlx::query_as(include_str!("../../queries/scan/create_artist.sql"))
                    .bind(artist)
                    .bind(metadata.artist_sort.as_ref().unwrap_or(artist))
                    .fetch_one(pool)
                    .await?;

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
) -> anyhow::Result<Option<i64>> {
    let Some(album) = &metadata.album else {
        return Ok(None);
    };

    let result: Result<(i64,), sqlx::Error> =
        sqlx::query_as(include_str!("../../queries/scan/get_album_id.sql"))
            .bind(album)
            .fetch_one(pool)
            .await;

    match result {
        Ok(v) => Ok(Some(v.0)),
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
                let downscaled = if size.0 <= 1024 || size.1 <= 1024 {
                    v.clone().to_vec()
                } else {
                    let resized = image::imageops::resize(
                        &decoded,
                        1024,
                        1024,
                        image::imageops::FilterType::Lanczos3,
                    );
                    let mut buf: Cursor<Vec<u8>> = Cursor::new(Vec::new());
                    let mut encoder = JpegEncoder::new_with_quality(&mut buf, 70);

                    encoder.encode(
                        resized.as_bytes(),
                        resized.width(),
                        resized.height(),
                        image::ExtendedColorType::Rgb8,
                    )?;
                    buf.flush()?;

                    buf.into_inner()
                };

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
                    .bind(album)
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

async fn scan_track(path: PathBuf, pool: SqlitePool) -> anyhow::Result<()> {
    if !file_is_scannable(path.as_ref(), &SymphoniaProvider::SUPPORTED_EXTENSIONS) {
        return Ok(());
    }

    let info = read_metadata_for_path(path.as_ref()).await?;

    let Some(artist_id) = insert_artist(&info.metadata, &pool).await? else {
        return Ok(());
    };

    let Some(album_id) = insert_album(&info.metadata, artist_id, &info.album_art, &pool).await?
    else {
        return Ok(());
    };

    insert_track(
        &info.metadata,
        album_id,
        path.as_ref(),
        info.duration,
        &pool,
    )
    .await?;

    Ok(())
}
