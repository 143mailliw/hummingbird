SELECT t.location FROM playlist_item AS pi
    JOIN track AS t ON pi.track_id = t.id
    WHERE pi.playlist_id = 1
    ORDER BY pi.position ASC;
