use std::borrow::Cow;

use anyhow::anyhow;
use smol::block_on;
use sqlx::SqlitePool;
use url::Url;

pub fn load(pool: &SqlitePool, url: Url) -> gpui::Result<Option<Cow<'static, [u8]>>> {
    match url.host_str().ok_or(anyhow!("missing table name"))? {
        "album" => {
            let mut segments = url.path_segments().ok_or(anyhow!("missing path"))?;
            let id = segments
                .next()
                .ok_or(anyhow!("missing id"))?
                .parse::<i64>()?;

            let image_type = segments.next().ok_or(anyhow!("missing image type"))?;

            let image = match image_type {
                "thumb" => block_on(
                    sqlx::query_as::<_, (Vec<u8>,)>(include_str!(
                        "../../../queries/assets/find_album_thumb.sql"
                    ))
                    .bind(id)
                    .fetch_one(pool),
                )?,
                "full" => block_on(
                    sqlx::query_as::<_, (Vec<u8>,)>(include_str!(
                        "../../../queries/assets/find_album_art.sql"
                    ))
                    .bind(id)
                    .fetch_one(pool),
                )?,
                _ => unimplemented!(),
            };

            Ok(Some(Cow::Owned(image.0)))
        }
        _ => Ok(None),
    }
}
