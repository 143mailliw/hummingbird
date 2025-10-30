use std::{fs::File, hash::Hasher, io::Cursor, sync::Arc};

use ahash::AHasher;
use async_lock::Mutex;
use gpui::{App, AppContext, Entity, Global, RenderImage, SharedString, Task};
use image::{Frame, ImageReader, imageops::thumbnail};
use smallvec::smallvec;
use tracing::{debug, error};

use crate::{
    media::{builtin::symphonia::SymphoniaProvider, traits::MediaProvider},
    playback::queue::{DataSource, QueueItemUIData},
    util::rgb_to_bgr,
};

#[derive(Clone)]
pub struct AlbumCache {
    pub cache: Arc<Mutex<moka::future::Cache<u64, Arc<RenderImage>>>>,
}

impl AlbumCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(moka::future::Cache::new(30))),
        }
    }
}

impl Global for AlbumCache {}

pub fn create_album_cache(cx: &mut App) {
    cx.set_global(AlbumCache::new());
}

fn decode_image(data: Box<[u8]>, thumb: bool) -> anyhow::Result<Arc<RenderImage>> {
    let mut image = ImageReader::new(Cursor::new(data))
        .with_guessed_format()?
        .decode()?
        .into_rgba8();

    rgb_to_bgr(&mut image);

    let frame = if thumb {
        Frame::new(thumbnail(&image, 80, 80))
    } else {
        Frame::new(image)
    };

    Ok(Arc::new(RenderImage::new(smallvec![frame])))
}

async fn read_metadata(path: String, cache: &mut AlbumCache) -> anyhow::Result<QueueItemUIData> {
    let file = File::open(&path)?;

    // TODO: Switch to a different media provider based on the file
    let mut media_provider = SymphoniaProvider::default();
    media_provider.open(file, None)?;
    media_provider.start_playback()?;

    let album_art_source = media_provider.read_image().ok().flatten();

    let album_art = if let Some(v) = album_art_source {
        let cache_lock = cache.cache.lock().await;

        // hash before hand to avoid storing the entire image as a key
        let mut hasher = AHasher::default();
        hasher.write(&v);
        let hash = hasher.finish();

        if let Some(image) = cache_lock.get(&hash).await {
            debug!("read_metadata cache hit for {}", hash);
            Some(image.clone())
        } else {
            let image = decode_image(v, true);
            if let Ok(image) = image {
                debug!("read_metadata cache miss for {}", hash);
                cache_lock.insert(hash, image.clone()).await;
                Some(image)
            } else if let Err(err) = image {
                error!("Failed to read image for metadata: {}", err);
                None
            } else {
                unreachable!()
            }
        }
    } else {
        None
    };

    let metadata = media_provider.read_metadata()?;

    Ok(QueueItemUIData {
        image: album_art,
        name: metadata.name.as_ref().map(SharedString::from),
        artist_name: metadata.artist.as_ref().map(SharedString::from),
        source: DataSource::Metadata,
    })
}

pub trait Decode {
    fn decode_image(
        &self,
        data: Box<[u8]>,
        thumb: bool,
        entity: Entity<Option<Arc<RenderImage>>>,
    ) -> Task<()>;
    fn read_metadata(&self, path: String, entity: Entity<Option<QueueItemUIData>>) -> Task<()>;
}

impl Decode for App {
    fn decode_image(
        &self,
        data: Box<[u8]>,
        thumb: bool,
        entity: Entity<Option<Arc<RenderImage>>>,
    ) -> Task<()> {
        self.spawn(async move |cx| {
            let decode_task = cx
                .background_spawn(async move { decode_image(data, thumb) })
                .await;

            let Ok(image) = decode_task else {
                error!("Failed to decode image - {:?}", decode_task);
                entity
                    .update(cx, |m, cx| {
                        *m = None;
                        cx.notify();
                    })
                    .expect("Failed to update entity");
                return;
            };

            entity
                .update(cx, |m, cx| {
                    *m = Some(image);
                    cx.notify();
                })
                .expect("Failed to update entity");
        })
    }

    fn read_metadata(&self, path: String, entity: Entity<Option<QueueItemUIData>>) -> Task<()> {
        let mut cache = self.global::<AlbumCache>().clone();

        self.spawn(async move |cx| {
            let read_task = cx
                .background_spawn(async move { read_metadata(path, &mut cache).await })
                .await;

            let Ok(metadata) = read_task else {
                error!("Failed to read metadata - {:?}", read_task);
                return;
            };

            entity
                .update(cx, |m, cx| {
                    *m = Some(metadata);
                    cx.notify();
                })
                .expect("Failed to update entity");
        })
    }
}
