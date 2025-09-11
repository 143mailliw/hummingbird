SELECT playlist.*, COUNT(playlist_item.id) as track_count FROM playlist JOIN playlist_item ON playlist.id = playlist_item.playlist_id;
