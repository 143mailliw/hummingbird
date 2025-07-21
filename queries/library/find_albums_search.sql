SELECT
    p.id,
    p.title,
    a.name
FROM
    album p JOIN artist a ON p.artist_id = a.id;
