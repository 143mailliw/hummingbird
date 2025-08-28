CREATE TABLE IF NOT EXISTS playlist_item (
    id INTEGER PRIMARY KEY,
    playlist_id INTEGER NOT NULL,
    track_id INTEGER NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    position INTEGER NOT NULL,
    FOREIGN KEY (playlist_id) REFERENCES playlist(id),
    FOREIGN KEY (track_id) REFERENCES track(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS playlist_item_playlist_id_track_id ON playlist_item(playlist_id, track_id);
