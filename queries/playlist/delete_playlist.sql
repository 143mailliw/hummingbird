DELETE FROM playlist_item WHERE playlist_id = $1;
DELETE FROM playlist WHERE id = $1;
