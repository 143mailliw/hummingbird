INSERT INTO album_path (album_id, path, disc_num)
    VALUES ($1, $2, $3)
    ON CONFLICT (album_id, disc_num) DO NOTHING;
