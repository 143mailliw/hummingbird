INSERT INTO album (title, title_sortable, artist_id, image, thumb, release_date, label, catalog_number, isrc)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
    ON CONFLICT (title, artist_id) DO NOTHING -- TODO: ideally we should have some way of updating this
    RETURNING id;
