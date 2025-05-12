ALTER TABLE album ADD folder TEXT;

CREATE TRIGGER IF NOT EXISTS delete_album_path_trigger AFTER DELETE ON track BEGIN
DELETE FROM album_path
WHERE
    album_path.path = OLD.folder
    AND album_path.disc_num = OLD.disc_number
    AND album_path.album_id = OLD.album_id
    AND NOT EXISTS (
        SELECT
            1
        FROM
            track
        WHERE
            track.folder = OLD.folder
            AND track.disc_number = OLD.disc_number
            AND track.album_id = OLD.album_id
    );

END;
