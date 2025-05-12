-- just set this to "none" if there isn't one for now
ALTER TABLE album ADD mbid TEXT DEFAULT "none" NOT NULL;

CREATE TABLE IF NOT EXISTS album_path (
    album_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    disc_num INTEGER DEFAULT -1 NOT NULL,
    FOREIGN KEY (album_id) REFERENCES album (id),
    PRIMARY KEY (album_id, disc_num)
);

DROP INDEX album_title_artist_id_idx;

CREATE UNIQUE INDEX IF NOT EXISTS album_title_artist_mbid ON album (title, artist_id, mbid);

CREATE TRIGGER IF NOT EXISTS delete_album_paths AFTER DELETE ON album BEGIN
DELETE FROM album_path
WHERE
    album_path.album_id = OLD.id;

END;
