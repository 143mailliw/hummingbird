use std::borrow::Cow;

use anyhow::anyhow;
use smol::block_on;
use sqlx::SqlitePool;
use url::Url;

pub fn load(pool: &SqlitePool, url: Url) -> gpui::Result<Option<Cow<'static, [u8]>>> {
    match url
        .host_str()
        .ok_or_else(|| anyhow!("missing table name"))?
    {
        "album" => {
            let mut segments = url.path_segments().ok_or_else(|| anyhow!("missing path"))?;
            let id: i64 = segments
                .next()
                .ok_or_else(|| anyhow!("missing id"))?
                .parse()?;
            let image_type = segments
                .next()
                .ok_or_else(|| anyhow!("missing image type"))?;

            let query = match image_type {
                "thumb" => include_str!("../../../queries/assets/find_album_thumb.sql"),
                "full" => include_str!("../../../queries/assets/find_album_art.sql"),
                _ => unimplemented!("invalid image type '{image_type}'"),
            };

            let (image,) = block_on(sqlx::query_as(query).bind(id).fetch_one(pool))?;
            Ok(Some(Cow::Owned(image)))
        }
        _ => Ok(None),
    }
}
