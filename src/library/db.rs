use std::{path::Path, sync::Arc};

use gpui::App;
use smol::block_on;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous},
};
use tracing::debug;

use crate::{
    library::types::{Playlist, PlaylistItem, PlaylistWithCount, TrackStats},
    ui::app::Pool,
};

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

/// Lists all albums for searching. Returns a vector of tuples containing the id, name, and artist
/// name.
pub async fn list_albums_search(
    pool: &SqlitePool,
) -> Result<Vec<(u32, String, String)>, sqlx::Error> {
    let query = include_str!("../../queries/library/find_albums_search.sql");

    let albums = sqlx::query_as::<_, (u32, String, String)>(query)
        .fetch_all(pool)
        .await?;

    Ok(albums)
}

pub async fn add_playlist_item(
    pool: &SqlitePool,
    playlist_id: i64,
    track_id: i64,
) -> Result<i64, sqlx::Error> {
    let query = include_str!("../../queries/playlist/add_track.sql");

    let id = sqlx::query(query)
        .bind(playlist_id)
        .bind(track_id)
        .execute(pool)
        .await?
        .last_insert_rowid();

    Ok(id)
}

pub async fn create_playlist(pool: &SqlitePool, name: &str) -> Result<i64, sqlx::Error> {
    let query = include_str!("../../queries/playlist/create_playlist.sql");

    let playlist_id = sqlx::query(query)
        .bind(name)
        .execute(pool)
        .await?
        .last_insert_rowid();

    Ok(playlist_id)
}

pub async fn delete_playlist(pool: &SqlitePool, playlist_id: i64) -> Result<(), sqlx::Error> {
    let query = include_str!("../../queries/playlist/delete_playlist.sql");

    sqlx::query(query).bind(playlist_id).execute(pool).await?;

    Ok(())
}

pub async fn get_all_playlists(
    pool: &SqlitePool,
) -> Result<Arc<Vec<PlaylistWithCount>>, sqlx::Error> {
    let query = include_str!("../../queries/playlist/get_all_playlists.sql");

    let playlists: Vec<PlaylistWithCount> = sqlx::query_as(query).fetch_all(pool).await?;

    Ok(Arc::new(playlists))
}

pub async fn get_playlist(
    pool: &SqlitePool,
    playlist_id: i64,
) -> Result<Arc<Playlist>, sqlx::Error> {
    let query = include_str!("../../queries/playlist/get_playlist.sql");

    let playlist: Playlist = sqlx::query_as(query)
        .bind(playlist_id)
        .fetch_one(pool)
        .await?;

    Ok(Arc::new(playlist))
}

pub async fn get_playlist_track_files(
    pool: &SqlitePool,
    playlist_id: i64,
) -> Result<Arc<Vec<String>>, sqlx::Error> {
    let query = include_str!("../../queries/playlist/get_track_files.sql");

    let track_files: Vec<(String,)> = sqlx::query_as(query)
        .bind(playlist_id)
        .fetch_all(pool)
        .await?;

    Ok(Arc::new(track_files.into_iter().map(|v| v.0).collect()))
}

/// Returns (playlist_item_id, track_id, album_id)
pub async fn get_playlist_tracks(
    pool: &SqlitePool,
    playlist_id: i64,
) -> Result<Arc<Vec<(i64, i64, i64)>>, sqlx::Error> {
    let query = include_str!("../../queries/playlist/get_track_listing.sql");

    let tracks: Vec<(i64, i64, i64)> = sqlx::query_as(query)
        .bind(playlist_id)
        .fetch_all(pool)
        .await?;

    Ok(Arc::new(tracks))
}

pub async fn move_playlist_item(
    pool: &SqlitePool,
    item_id: i64,
    new_position: i64,
) -> Result<(), sqlx::Error> {
    // retrieve the current item's position
    let original_item = get_playlist_item(pool, item_id).await?;

    if original_item.position > new_position {
        let move_query = include_str!("../../queries/playlist/move_track_down.sql");

        sqlx::query(move_query)
            .bind(new_position)
            .bind(original_item.position)
            .bind(item_id)
            .execute(pool)
            .await?;
    } else if original_item.position < new_position {
        let move_query = include_str!("../../queries/playlist/move_track_up.sql");

        sqlx::query(move_query)
            .bind(new_position)
            .bind(original_item.position)
            .bind(item_id)
            .execute(pool)
            .await?;
    }

    Ok(())
}

pub async fn remove_playlist_item(pool: &SqlitePool, item_id: i64) -> Result<(), sqlx::Error> {
    let query = include_str!("../../queries/playlist/remove_track.sql");
    let item = get_playlist_item(pool, item_id).await?;

    sqlx::query(query)
        .bind(item.position)
        .bind(item_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_playlist_item(
    pool: &SqlitePool,
    item_id: i64,
) -> Result<PlaylistItem, sqlx::Error> {
    let query = include_str!("../../queries/playlist/select_playlist_item.sql");

    let item: PlaylistItem = sqlx::query_as(query).bind(item_id).fetch_one(pool).await?;

    Ok(item)
}

pub async fn get_track_stats(pool: &SqlitePool) -> Result<Arc<TrackStats>, sqlx::Error> {
    let query = include_str!("../../queries/track_stats.sql");

    let stats: TrackStats = sqlx::query_as(query).fetch_one(pool).await?;

    Ok(Arc::new(stats))
}

pub async fn playlist_has_track(
    pool: &SqlitePool,
    playlist_id: i64,
    track_id: i64,
) -> Result<Option<i64>, sqlx::Error> {
    let query = include_str!("../../queries/playlist/playlist_has_track.sql");

    let has_track: Option<i64> = sqlx::query_scalar(query)
        .bind(playlist_id)
        .bind(track_id)
        .fetch_optional(pool)
        .await?;

    Ok(has_track)
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
    fn list_albums_search(&self) -> Result<Vec<(u32, String, String)>, sqlx::Error>;
    fn add_playlist_item(&self, playlist_id: i64, track_id: i64) -> Result<i64, sqlx::Error>;
    fn create_playlist(&self, name: &str) -> Result<i64, sqlx::Error>;
    fn delete_playlist(&self, playlist_id: i64) -> Result<(), sqlx::Error>;
    fn get_all_playlists(&self) -> Result<Arc<Vec<PlaylistWithCount>>, sqlx::Error>;
    fn get_playlist(&self, playlist_id: i64) -> Result<Arc<Playlist>, sqlx::Error>;
    fn get_playlist_track_files(&self, playlist_id: i64) -> Result<Arc<Vec<String>>, sqlx::Error>;
    fn get_playlist_tracks(
        &self,
        playlist_id: i64,
    ) -> Result<Arc<Vec<(i64, i64, i64)>>, sqlx::Error>;
    fn move_playlist_item(&self, item_id: i64, new_position: i64) -> Result<(), sqlx::Error>;
    fn remove_playlist_item(&self, item_id: i64) -> Result<(), sqlx::Error>;
    fn get_playlist_item(&self, item_id: i64) -> Result<PlaylistItem, sqlx::Error>;
    fn get_track_stats(&self) -> Result<Arc<TrackStats>, sqlx::Error>;
    fn playlist_has_track(
        &self,
        playlist_id: i64,
        track_id: i64,
    ) -> Result<Option<i64>, sqlx::Error>;
}

impl LibraryAccess for App {
    fn list_albums(&self, sort_method: AlbumSortMethod) -> Result<Vec<(u32, String)>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(list_albums(&pool.0, sort_method))
    }

    fn list_tracks_in_album(&self, album_id: i64) -> Result<Arc<Vec<Track>>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(list_tracks_in_album(&pool.0, album_id))
    }

    fn get_album_by_id(
        &self,
        album_id: i64,
        method: AlbumMethod,
    ) -> Result<Arc<Album>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(get_album_by_id(&pool.0, album_id, method))
    }

    fn get_artist_name_by_id(&self, artist_id: i64) -> Result<Arc<String>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(get_artist_name_by_id(&pool.0, artist_id))
    }

    fn get_artist_by_id(&self, artist_id: i64) -> Result<Arc<Artist>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(get_artist_by_id(&pool.0, artist_id))
    }

    fn get_track_by_id(&self, track_id: i64) -> Result<Arc<Track>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(get_track_by_id(&pool.0, track_id))
    }

    /// Lists all albums for searching. Returns a vector of tuples containing the id, name, and artist
    /// name.
    fn list_albums_search(&self) -> Result<Vec<(u32, String, String)>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(list_albums_search(&pool.0))
    }

    fn add_playlist_item(&self, playlist_id: i64, track_id: i64) -> Result<i64, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(add_playlist_item(&pool.0, playlist_id, track_id))
    }

    fn create_playlist(&self, name: &str) -> Result<i64, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(create_playlist(&pool.0, name))
    }

    fn delete_playlist(&self, playlist_id: i64) -> Result<(), sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(delete_playlist(&pool.0, playlist_id))
    }

    fn get_all_playlists(&self) -> Result<Arc<Vec<PlaylistWithCount>>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(get_all_playlists(&pool.0))
    }

    fn get_playlist(&self, playlist_id: i64) -> Result<Arc<Playlist>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(get_playlist(&pool.0, playlist_id))
    }

    fn get_playlist_track_files(&self, playlist_id: i64) -> Result<Arc<Vec<String>>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(get_playlist_track_files(&pool.0, playlist_id))
    }

    fn get_playlist_tracks(
        &self,
        playlist_id: i64,
    ) -> Result<Arc<Vec<(i64, i64, i64)>>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(get_playlist_tracks(&pool.0, playlist_id))
    }

    fn move_playlist_item(&self, item_id: i64, new_position: i64) -> Result<(), sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(move_playlist_item(&pool.0, item_id, new_position))
    }

    fn remove_playlist_item(&self, item_id: i64) -> Result<(), sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(remove_playlist_item(&pool.0, item_id))
    }

    fn get_playlist_item(&self, item_id: i64) -> Result<PlaylistItem, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(get_playlist_item(&pool.0, item_id))
    }

    fn get_track_stats(&self) -> Result<Arc<TrackStats>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(get_track_stats(&pool.0))
    }

    fn playlist_has_track(
        &self,
        playlist_id: i64,
        track_id: i64,
    ) -> Result<Option<i64>, sqlx::Error> {
        let pool: &Pool = self.global();
        block_on(playlist_has_track(&pool.0, playlist_id, track_id))
    }
}
