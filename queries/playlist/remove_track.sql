UPDATE playlist_item SET position = position - 1 WHERE position > $1;
DELETE FROM playlist_item WHERE id = $2
