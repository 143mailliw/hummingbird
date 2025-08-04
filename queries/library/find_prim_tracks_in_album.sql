SELECT id, track_number, disc_number, album_id, location FROM track
WHERE album_id = $1
ORDER BY disc_number ASC, track_number ASC;
