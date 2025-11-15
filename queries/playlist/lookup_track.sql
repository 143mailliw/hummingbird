SELECT t.id FROM track t
    JOIN main.album a ON a.id = t.album_id
    JOIN main.artist a2 ON a2.id = a.artist_id
    WHERE (t.location LIKE $1)
        OR (
                $2 IS NOT NULL
            AND t.title = $2
            AND ($3 IS NULL OR a2.name = $3)
            AND ($4 IS NULL OR a.title = $4)
            AND ($5 IS NULL OR t.duration = $5 )
        );
