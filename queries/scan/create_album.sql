INSERT INTO album (title, title_sortable, artist_id, image, thumb, release_date, release_year, label, catalog_number, isrc, mbid)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    ON CONFLICT (title, artist_id, mbid) DO UPDATE SET
        title = EXCLUDED.title,
        title_sortable = EXCLUDED.title_sortable,
        artist_id = EXCLUDED.artist_id,
        image = EXCLUDED.image,
        thumb = EXCLUDED.thumb,
        release_date = EXCLUDED.release_date,
        release_year = EXCLUDED.release_year,
        label = EXCLUDED.label,
        catalog_number = EXCLUDED.catalog_number,
        isrc = EXCLUDED.isrc,
        mbid = EXCLUDED.mbid
    RETURNING id;
