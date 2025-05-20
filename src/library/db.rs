use std::{path::Path, sync::Arc};

use async_std::task;
use gpui::App;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
    SqlitePool,
};
use tracing::debug;

use crate::ui::app::Pool;

use super::types::{Album, Artist, Track};

pub async fn create_pool(path: impl AsRef<Path>) -> Result<SqlitePool, sqlx::Error> {
    debug!("Creating database pool at {:?}", path.as_ref());
    let options = SqliteConnectOptions::new()
        .filename(path)
        .optimize_on_close(true, None)
        .synchronous(SqliteSynchronous::Normal)
        .journal_mode(SqliteJournalMode::Wal)
        .statement_cache_capacity(0)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;

    sqlx::query("PRAGMA mmap_size = 30000000000")
        .execute(&pool)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlbumSortMethod {
    TitleAsc,
    TitleDesc,
    ArtistAsc,
    ArtistDesc,
    ReleaseAsc,
    ReleaseDesc,
    LabelAsc,
    LabelDesc,
    CatalogAsc,
    CatalogDesc,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlbumMethod {
    FullQuality,
    Thumbnail,
}

pub async fn list_albums(
    pool: &SqlitePool,
    sort_method: AlbumSortMethod,
) -> Result<Vec<(u32, String)>, sqlx::Error> {
    let query = match sort_method {
        AlbumSortMethod::TitleAsc => {
            include_str!("../../queries/library/find_albums_title_asc.sql")
        }
        AlbumSortMethod::TitleDesc => {
            include_str!("../../queries/library/find_albums_title_desc.sql")
        }
        AlbumSortMethod::ArtistAsc => {
            include_str!("../../queries/library/find_albums_artist_asc.sql")
        }
        AlbumSortMethod::ArtistDesc => {
            include_str!("../../queries/library/find_albums_artist_desc.sql")
        }
        AlbumSortMethod::ReleaseAsc => {
            include_str!("../../queries/library/find_albums_release_asc.sql")
        }
        AlbumSortMethod::ReleaseDesc => {
            include_str!("../../queries/library/find_albums_release_desc.sql")
        }
        AlbumSortMethod::LabelAsc => {
            include_str!("../../queries/library/find_albums_label_asc.sql")
        }
        AlbumSortMethod::LabelDesc => {
            include_str!("../../queries/library/find_albums_label_desc.sql")
        }
        AlbumSortMethod::CatalogAsc => {
            include_str!("../../queries/library/find_albums_catnum_asc.sql")
        }
        AlbumSortMethod::CatalogDesc => {
            include_str!("../../queries/library/find_albums_catnum_desc.sql")
        }
    };

    let albums = sqlx::query_as::<_, (u32, String)>(query)
        .fetch_all(pool)
        .await?;

    Ok(albums)
}

pub async fn list_tracks_in_album(
    pool: &SqlitePool,
    album_id: i64,
) -> Result<Arc<Vec<Track>>, sqlx::Error> {
    let query = include_str!("../../queries/library/find_tracks_in_album.sql");

    let albums = Arc::new(
        sqlx::query_as::<_, Track>(query)
            .bind(album_id)
            .fetch_all(pool)
            .await?,
    );

    Ok(albums)
}

pub async fn get_album_by_id(
    pool: &SqlitePool,
    album_id: i64,
    method: AlbumMethod,
) -> Result<Arc<Album>, sqlx::Error> {
    let query = include_str!("../../queries/library/find_album_by_id.sql");

    let album: Arc<Album> = Arc::new({
        let mut data: Album = sqlx::query_as(query).bind(album_id).fetch_one(pool).await?;

        match method {
            AlbumMethod::FullQuality => {
                data.thumb = None;
            }
            AlbumMethod::Thumbnail => {
                data.image = None;
            }
        }

        data
    });

    Ok(album)
}

pub async fn get_artist_name_by_id(
    pool: &SqlitePool,
    artist_id: i64,
) -> Result<Arc<String>, sqlx::Error> {
    let query = include_str!("../../queries/library/find_artist_name_by_id.sql");

    let artist_name: Arc<String> = Arc::new(
        sqlx::query_scalar(query)
            .bind(artist_id)
            .fetch_one(pool)
            .await?,
    );

    Ok(artist_name)
}

pub async fn get_artist_by_id(
    pool: &SqlitePool,
    artist_id: i64,
) -> Result<Arc<Artist>, sqlx::Error> {
    let query = include_str!("../../queries/library/find_artist_by_id.sql");

    let artist: Arc<Artist> = Arc::new(
        sqlx::query_as(query)
            .bind(artist_id)
            .fetch_one(pool)
            .await?,
    );

    Ok(artist)
}

pub async fn get_track_by_id(pool: &SqlitePool, track_id: i64) -> Result<Arc<Track>, sqlx::Error> {
    let query = include_str!("../../queries/library/find_track_by_id.sql");

    let track: Arc<Track> = Arc::new(sqlx::query_as(query).bind(track_id).fetch_one(pool).await?);

    Ok(track)
}

pub async fn list_albums_search(pool: &SqlitePool) -> Result<Vec<(u32, String)>, sqlx::Error> {
    let query = include_str!("../../queries/library/find_albums_search.sql");

    let albums = sqlx::query_as::<_, (u32, String)>(query)
        .fetch_all(pool)
        .await?;

    Ok(albums)
}

pub trait LibraryAccess {
    fn list_albums(&self, sort_method: AlbumSortMethod) -> Result<Vec<(u32, String)>, sqlx::Error>;
    fn list_tracks_in_album(&self, album_id: i64) -> Result<Arc<Vec<Track>>, sqlx::Error>;
    fn get_album_by_id(
        &self,
        album_id: i64,
        method: AlbumMethod,
    ) -> Result<Arc<Album>, sqlx::Error>;
    fn get_artist_name_by_id(&self, artist_id: i64) -> Result<Arc<String>, sqlx::Error>;
    fn get_artist_by_id(&self, artist_id: i64) -> Result<Arc<Artist>, sqlx::Error>;
    fn get_track_by_id(&self, track_id: i64) -> Result<Arc<Track>, sqlx::Error>;
    fn list_albums_search(&self) -> Result<Vec<(u32, String)>, sqlx::Error>;
}

impl LibraryAccess for App {
    fn list_albums(&self, sort_method: AlbumSortMethod) -> Result<Vec<(u32, String)>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(list_albums(&pool.0, sort_method))
    }

    fn list_tracks_in_album(&self, album_id: i64) -> Result<Arc<Vec<Track>>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(list_tracks_in_album(&pool.0, album_id))
    }

    fn get_album_by_id(
        &self,
        album_id: i64,
        method: AlbumMethod,
    ) -> Result<Arc<Album>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(get_album_by_id(&pool.0, album_id, method))
    }

    fn get_artist_name_by_id(&self, artist_id: i64) -> Result<Arc<String>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(get_artist_name_by_id(&pool.0, artist_id))
    }

    fn get_artist_by_id(&self, artist_id: i64) -> Result<Arc<Artist>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(get_artist_by_id(&pool.0, artist_id))
    }

    fn get_track_by_id(&self, track_id: i64) -> Result<Arc<Track>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(get_track_by_id(&pool.0, track_id))
    }

    fn list_albums_search(&self) -> Result<Vec<(u32, String)>, sqlx::Error> {
        let pool: &Pool = self.global();
        task::block_on(list_albums_search(&pool.0))
    }
}
