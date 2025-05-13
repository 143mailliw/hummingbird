use chrono::{DateTime, Utc};

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Metadata {
    pub name: Option<String>,
    pub artist: Option<String>,
    pub album_artist: Option<String>,
    pub artist_sort: Option<String>,
    pub original_artist: Option<String>,
    pub composer: Option<String>,
    pub album: Option<String>,
    pub sort_album: Option<String>,
    pub genre: Option<String>,
    pub grouping: Option<String>,
    pub bpm: Option<u64>,
    pub compilation: bool,
    pub date: Option<DateTime<Utc>>,

    pub track_current: Option<u64>,
    pub track_max: Option<u64>,
    pub disc_current: Option<u64>,
    pub disc_max: Option<u64>,

    pub label: Option<String>,
    pub catalog: Option<String>,
    pub isrc: Option<String>,

    pub mbid_album: Option<String>,
}
