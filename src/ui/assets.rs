pub mod bundled;
pub mod db;

use std::borrow::Cow;

use gpui::AssetSource;
use sqlx::SqlitePool;
use url::Url;

use crate::ui::assets::bundled::BundledAssets;

pub struct HummingbirdAssetSource {
    pool: SqlitePool,
}

impl HummingbirdAssetSource {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl AssetSource for HummingbirdAssetSource {
    fn load(&self, path: &str) -> gpui::Result<Option<Cow<'static, [u8]>>> {
        let url = Url::parse(&path[1..])?;

        match url.scheme() {
            "db" => db::load(&self.pool, url),
            "bundled" => BundledAssets::load(url),
            _ => panic!("invalid url scheme for resource"),
        }
    }

    fn list(&self, path: &str) -> gpui::Result<Vec<gpui::SharedString>> {
        BundledAssets.list(path)
    }
}
