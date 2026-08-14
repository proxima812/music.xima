//! `TrackRepository` on SQLite. Owns the scanner write path, including the
//! `tracks_fts` bookkeeping (CONTRACTS §3, §4).

use sqlx::sqlite::SqliteConnection;
use sqlx::{QueryBuilder, Row, Sqlite};

use crate::domain::{
    clamp_limit, normalize_sort_key, LibraryStats, Page, ScannedTrack, Track, TrackQuery,
};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::TrackRepository;

use super::pool::Db;
use super::sql::{
    conjunction, dyn_query, fts_delete, fts_insert, fts_payload, track_from_row, track_order_by,
    track_select, tracks_by_ids, BIND_CHUNK, TRACK_JOINS,
};

pub struct SqliteTrackRepository {
    pool: Db,
}

impl SqliteTrackRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }
}

/// Appends the filters of a library query. Every value is bound, never
/// formatted into the statement.
fn push_filters(builder: &mut QueryBuilder<Sqlite>, query: &TrackQuery) {
    let mut has_where = false;

    if let Some(artist_id) = query.artist_id {
        builder.push(conjunction(&mut has_where));
        builder.push("t.artist_id = ").push_bind(artist_id);
    }
    if let Some(album_id) = query.album_id {
        builder.push(conjunction(&mut has_where));
        builder.push("t.album_id = ").push_bind(album_id);
    }
    if let Some(genre) = query.genre.as_deref() {
        builder.push(conjunction(&mut has_where));
        builder
            .push("t.genre = ")
            .push_bind(genre.to_owned())
            .push(" COLLATE NOCASE");
    }
    if let Some(folder) = query.folder.as_deref() {
        builder.push(conjunction(&mut has_where));
        builder.push("t.folder = ").push_bind(folder.to_owned());
    }
    if query.favorites_only {
        builder.push(conjunction(&mut has_where));
        builder.push("f.track_id IS NOT NULL");
    }
}

/// Finds or creates the artist row for `name`, keyed by its sort name.
/// `None` for blank names — a track without an artist keeps a NULL column.
async fn upsert_artist(conn: &mut SqliteConnection, name: Option<&str>) -> CoreResult<Option<i64>> {
    let Some(name) = name.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let sort_name = normalize_sort_key(name);
    if sort_name.is_empty() {
        return Ok(None);
    }

    if let Some(row) = sqlx::query("SELECT id FROM artists WHERE sort_name = ?")
        .bind(&sort_name)
        .fetch_optional(&mut *conn)
        .await?
    {
        return Ok(Some(row.try_get("id")?));
    }

    let result = sqlx::query("INSERT INTO artists (name, sort_name) VALUES (?, ?)")
        .bind(name)
        .bind(&sort_name)
        .execute(&mut *conn)
        .await?;
    Ok(Some(result.last_insert_rowid()))
}

/// Finds or creates the album row for `(sort_title, artist_id)`. Year and
/// cover are filled in only while they are still unknown, so a later track with
/// missing tags never blanks them.
async fn upsert_album(
    conn: &mut SqliteConnection,
    title: Option<&str>,
    artist_id: Option<i64>,
    year: Option<i32>,
    cover_key: Option<&str>,
) -> CoreResult<Option<i64>> {
    let Some(title) = title.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let sort_title = normalize_sort_key(title);

    // `artist_id IS ?` and not `=`, so that albums without an artist match each
    // other instead of piling up duplicates (SQL NULL is never equal to NULL).
    if let Some(row) = sqlx::query("SELECT id FROM albums WHERE sort_title = ? AND artist_id IS ?")
        .bind(&sort_title)
        .bind(artist_id)
        .fetch_optional(&mut *conn)
        .await?
    {
        let id: i64 = row.try_get("id")?;
        sqlx::query(
            "UPDATE albums SET year = COALESCE(year, ?), cover_key = COALESCE(cover_key, ?) \
             WHERE id = ?",
        )
        .bind(year)
        .bind(cover_key)
        .bind(id)
        .execute(&mut *conn)
        .await?;
        return Ok(Some(id));
    }

    let result = sqlx::query(
        "INSERT INTO albums (artist_id, title, sort_title, year, cover_key) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(artist_id)
    .bind(title)
    .bind(&sort_title)
    .bind(year)
    .bind(cover_key)
    .execute(&mut *conn)
    .await?;
    Ok(Some(result.last_insert_rowid()))
}

/// Rebuilds the FTS row of `track_id` from the current table contents. The old
/// row, when there is one, must have been removed first.
async fn index_track(conn: &mut SqliteConnection, track_id: i64) -> CoreResult<()> {
    if let Some(payload) = fts_payload(conn, track_id).await? {
        fts_insert(conn, &payload).await?;
    }
    Ok(())
}

/// Drops the FTS row of `track_id` if the track exists.
async fn deindex_track(conn: &mut SqliteConnection, track_id: i64) -> CoreResult<()> {
    if let Some(payload) = fts_payload(conn, track_id).await? {
        fts_delete(conn, &payload).await?;
    }
    Ok(())
}

fn clean_text(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

#[async_trait::async_trait]
impl TrackRepository for SqliteTrackRepository {
    async fn get(&self, id: i64) -> CoreResult<Track> {
        let row = dyn_query(format!("{} WHERE t.id = ?", track_select()))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| CoreError::not_found("track", id))?;
        Ok(track_from_row(&row)?)
    }

    async fn get_many(&self, ids: &[i64]) -> CoreResult<Vec<Track>> {
        tracks_by_ids(&self.pool, ids).await
    }

    async fn query(&self, q: &TrackQuery) -> CoreResult<Page<Track>> {
        let query = q.normalized();

        let mut counter = QueryBuilder::<Sqlite>::new(format!(
            "SELECT COUNT(*) AS total FROM tracks t {TRACK_JOINS}"
        ));
        push_filters(&mut counter, &query);
        let total: i64 = counter
            .build()
            .fetch_one(&self.pool)
            .await?
            .try_get("total")?;

        if total <= query.offset {
            return Ok(Page::new(Vec::new(), total, query.offset, query.limit));
        }

        let mut builder = QueryBuilder::<Sqlite>::new(track_select());
        push_filters(&mut builder, &query);
        builder.push(" ORDER BY ").push(track_order_by(query.sort));
        builder.push(" LIMIT ").push_bind(query.limit);
        builder.push(" OFFSET ").push_bind(query.offset);

        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            items.push(track_from_row(row)?);
        }
        Ok(Page::new(items, total, query.offset, query.limit))
    }

    async fn recently_added(&self, limit: i64) -> CoreResult<Vec<Track>> {
        let rows = dyn_query(format!(
            "{} ORDER BY t.date_added DESC, t.id DESC LIMIT ?",
            track_select()
        ))
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;

        let mut tracks = Vec::with_capacity(rows.len());
        for row in &rows {
            tracks.push(track_from_row(row)?);
        }
        Ok(tracks)
    }

    async fn upsert_many(&self, tracks: &[ScannedTrack]) -> CoreResult<u64> {
        if tracks.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;
        let mut written: u64 = 0;

        for scanned in tracks {
            let uri = scanned.uri.trim();
            if uri.is_empty() {
                continue;
            }

            let title = match scanned.title.trim() {
                "" => uri.rsplit('/').next().unwrap_or(uri),
                value => value,
            };
            let sort_title = normalize_sort_key(title);

            let artist_id = upsert_artist(&mut tx, scanned.artist.as_deref()).await?;
            let album_artist_id = upsert_artist(&mut tx, scanned.effective_album_artist()).await?;
            let album_id = upsert_album(
                &mut tx,
                scanned.album.as_deref(),
                album_artist_id.or(artist_id),
                scanned.year,
                scanned.cover_key.as_deref(),
            )
            .await?;

            let genre = clean_text(scanned.genre.as_deref());
            let folder = clean_text(scanned.folder.as_deref());
            let format = scanned.format_label();

            let existing: Option<i64> = sqlx::query("SELECT id FROM tracks WHERE uri = ?")
                .bind(uri)
                .fetch_optional(&mut *tx)
                .await?
                .map(|row| row.try_get("id"))
                .transpose()?;

            let track_id = match existing {
                Some(id) => {
                    deindex_track(&mut tx, id).await?;
                    sqlx::query(
                        "UPDATE tracks SET \
                           title = ?, sort_title = ?, artist_id = ?, album_id = ?, \
                           duration_ms = ?, track_number = ?, disc_number = ?, year = ?, \
                           genre = ?, bitrate = ?, sample_rate = ?, size = ?, format = ?, \
                           cover_key = COALESCE(?, cover_key), folder = ?, last_modified = ? \
                         WHERE id = ?",
                    )
                    .bind(title)
                    .bind(&sort_title)
                    .bind(artist_id)
                    .bind(album_id)
                    .bind(scanned.duration_ms)
                    .bind(scanned.track_number)
                    .bind(scanned.disc_number)
                    .bind(scanned.year)
                    .bind(genre.as_deref())
                    .bind(scanned.bitrate)
                    .bind(scanned.sample_rate)
                    .bind(scanned.size)
                    .bind(format.as_deref())
                    .bind(scanned.cover_key.as_deref())
                    .bind(folder.as_deref())
                    .bind(scanned.last_modified)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
                    id
                }
                None => {
                    let result = sqlx::query(
                        "INSERT INTO tracks ( \
                           uri, title, sort_title, artist_id, album_id, duration_ms, \
                           track_number, disc_number, year, genre, bitrate, sample_rate, \
                           size, format, cover_key, folder, date_added, last_modified \
                         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    )
                    .bind(uri)
                    .bind(title)
                    .bind(&sort_title)
                    .bind(artist_id)
                    .bind(album_id)
                    .bind(scanned.duration_ms)
                    .bind(scanned.track_number)
                    .bind(scanned.disc_number)
                    .bind(scanned.year)
                    .bind(genre.as_deref())
                    .bind(scanned.bitrate)
                    .bind(scanned.sample_rate)
                    .bind(scanned.size)
                    .bind(format.as_deref())
                    .bind(scanned.cover_key.as_deref())
                    .bind(folder.as_deref())
                    .bind(scanned.date_added)
                    .bind(scanned.last_modified)
                    .execute(&mut *tx)
                    .await?;
                    result.last_insert_rowid()
                }
            };

            index_track(&mut tx, track_id).await?;
            written += 1;
        }

        tx.commit().await?;
        Ok(written)
    }

    async fn delete_missing(&self, keep_uris: &[String]) -> CoreResult<u64> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("CREATE TEMP TABLE IF NOT EXISTS scan_keep_uris (uri TEXT PRIMARY KEY)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM scan_keep_uris")
            .execute(&mut *tx)
            .await?;

        for chunk in keep_uris.chunks(BIND_CHUNK) {
            let mut builder =
                QueryBuilder::<Sqlite>::new("INSERT OR IGNORE INTO scan_keep_uris (uri) ");
            builder.push_values(chunk, |mut row, uri| {
                row.push_bind(uri.trim().to_owned());
            });
            builder.build().execute(&mut *tx).await?;
        }

        let doomed: Vec<i64> =
            sqlx::query("SELECT id FROM tracks WHERE uri NOT IN (SELECT uri FROM scan_keep_uris)")
                .fetch_all(&mut *tx)
                .await?
                .iter()
                .map(|row| row.try_get("id"))
                .collect::<Result<_, _>>()?;

        for id in &doomed {
            deindex_track(&mut tx, *id).await?;
        }

        let removed =
            sqlx::query("DELETE FROM tracks WHERE uri NOT IN (SELECT uri FROM scan_keep_uris)")
                .execute(&mut *tx)
                .await?
                .rows_affected();

        // Albums first: they reference artists.
        sqlx::query(
            "DELETE FROM albums WHERE id NOT IN \
             (SELECT album_id FROM tracks WHERE album_id IS NOT NULL)",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM artists WHERE id NOT IN \
               (SELECT artist_id FROM tracks WHERE artist_id IS NOT NULL) \
             AND id NOT IN \
               (SELECT artist_id FROM albums WHERE artist_id IS NOT NULL)",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query("DROP TABLE IF EXISTS scan_keep_uris")
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(removed)
    }

    async fn stats(&self) -> CoreResult<LibraryStats> {
        let row = sqlx::query(
            "SELECT \
               (SELECT COUNT(*) FROM tracks) AS tracks, \
               (SELECT COUNT(*) FROM albums) AS albums, \
               (SELECT COUNT(*) FROM artists) AS artists, \
               (SELECT COUNT(*) FROM playlists) AS playlists, \
               (SELECT COUNT(DISTINCT genre) FROM tracks \
                  WHERE genre IS NOT NULL AND TRIM(genre) <> '') AS genres, \
               (SELECT COALESCE(SUM(duration_ms), 0) FROM tracks) AS total_duration_ms, \
               (SELECT COALESCE(SUM(size), 0) FROM tracks) AS total_size",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(LibraryStats {
            tracks: row.try_get("tracks")?,
            albums: row.try_get("albums")?,
            artists: row.try_get("artists")?,
            playlists: row.try_get("playlists")?,
            genres: row.try_get("genres")?,
            total_duration_ms: row.try_get("total_duration_ms")?,
            total_size: row.try_get("total_size")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteTrackRepository;
    use crate::domain::{TrackQuery, TrackSort};
    use crate::infrastructure::repositories::TrackRepository;
    use crate::infrastructure::sqlite::sql::test_support::{full, pool, scanned};
    use sqlx::Row;

    #[tokio::test]
    async fn upsert_inserts_then_updates_by_uri() {
        let db = pool().await;
        let repo = SqliteTrackRepository::new(db.clone());

        let written = repo
            .upsert_many(&[full("content://1", "Alpha", "Radiohead", "The Bends")])
            .await
            .expect("first scan");
        assert_eq!(written, 1);

        let mut again = full("content://1", "Alpha Remastered", "Radiohead", "The Bends");
        again.duration_ms = 999;
        let written = repo.upsert_many(&[again]).await.expect("second scan");
        assert_eq!(written, 1);

        let page = repo
            .query(&TrackQuery::default())
            .await
            .expect("library query");
        assert_eq!(page.total, 1);
        let track = page.items.first().expect("one track");
        assert_eq!(track.title, "Alpha Remastered");
        assert_eq!(track.duration_ms, 999);
        assert_eq!(track.artist_name.as_deref(), Some("Radiohead"));
        assert_eq!(track.album_title.as_deref(), Some("The Bends"));
        assert_eq!(track.format.as_deref(), Some("MP3"));
        assert!(!track.is_favorite);
        assert_eq!(track.play_count, 0);

        let artists: i64 = sqlx::query("SELECT COUNT(*) AS n FROM artists")
            .fetch_one(&db)
            .await
            .expect("artists counted")
            .get("n");
        assert_eq!(artists, 1, "the artist must be reused across scans");

        let fts: i64 = sqlx::query("SELECT COUNT(*) AS n FROM tracks_fts")
            .fetch_one(&db)
            .await
            .expect("fts counted")
            .get("n");
        assert_eq!(fts, 1, "the fts row must be replaced, not duplicated");
    }

    #[tokio::test]
    async fn albums_without_an_artist_are_deduplicated() {
        let db = pool().await;
        let repo = SqliteTrackRepository::new(db.clone());

        let mut first = scanned("content://1", "One");
        first.album = Some("Untitled".to_owned());
        let mut second = scanned("content://2", "Two");
        second.album = Some("untitled".to_owned());

        repo.upsert_many(&[first, second]).await.expect("scan");

        let albums: i64 = sqlx::query("SELECT COUNT(*) AS n FROM albums")
            .fetch_one(&db)
            .await
            .expect("albums counted")
            .get("n");
        assert_eq!(albums, 1);
    }

    #[tokio::test]
    async fn query_sorts_filters_and_paginates() {
        let db = pool().await;
        let repo = SqliteTrackRepository::new(db.clone());

        let mut a = full("content://a", "Bravo", "Beta", "Second");
        a.date_added = 200;
        a.genre = Some("Jazz".to_owned());
        let mut b = full("content://b", "Alpha", "Alpha Artist", "First");
        b.date_added = 300;
        let mut c = full("content://c", "Charlie", "Gamma", "Third");
        c.date_added = 100;
        repo.upsert_many(&[a, b, c]).await.expect("scan");

        let by_title = repo
            .query(&TrackQuery::default())
            .await
            .expect("title sort");
        let titles: Vec<&str> = by_title.items.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, vec!["Alpha", "Bravo", "Charlie"]);
        assert_eq!(by_title.total, 3);

        let by_date = repo
            .query(&TrackQuery {
                sort: TrackSort::DateAddedDesc,
                ..TrackQuery::default()
            })
            .await
            .expect("date sort");
        let titles: Vec<&str> = by_date.items.iter().map(|t| t.title.as_str()).collect();
        assert_eq!(titles, vec!["Alpha", "Bravo", "Charlie"]);

        let page = repo
            .query(&TrackQuery {
                offset: 1,
                limit: 1,
                ..TrackQuery::default()
            })
            .await
            .expect("second page");
        assert_eq!(page.total, 3);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].title, "Bravo");
        assert!(page.has_more());

        let jazz = repo
            .query(&TrackQuery {
                genre: Some("jazz".to_owned()),
                ..TrackQuery::default()
            })
            .await
            .expect("genre filter");
        assert_eq!(jazz.total, 1);
        assert_eq!(jazz.items[0].title, "Bravo");

        let folder = repo
            .query(&TrackQuery {
                folder: Some("Music/Rock".to_owned()),
                ..TrackQuery::default()
            })
            .await
            .expect("folder filter");
        assert_eq!(folder.total, 3);

        let none = repo
            .query(&TrackQuery {
                offset: 500,
                ..TrackQuery::default()
            })
            .await
            .expect("past the end");
        assert!(none.items.is_empty());
        assert_eq!(none.total, 3);
    }

    #[tokio::test]
    async fn get_many_keeps_the_requested_order() {
        let db = pool().await;
        let repo = SqliteTrackRepository::new(db.clone());
        repo.upsert_many(&[
            full("content://a", "Alpha", "A", "A"),
            full("content://b", "Bravo", "B", "B"),
        ])
        .await
        .expect("scan");

        let all = repo.query(&TrackQuery::default()).await.expect("query");
        let first = all.items[0].id;
        let second = all.items[1].id;

        let ordered = repo
            .get_many(&[second, first, 9_999])
            .await
            .expect("get_many");
        assert_eq!(
            ordered.iter().map(|t| t.id).collect::<Vec<_>>(),
            vec![second, first]
        );
        assert!(repo.get_many(&[]).await.expect("empty").is_empty());
    }

    #[tokio::test]
    async fn missing_track_is_reported_as_not_found() {
        let repo = SqliteTrackRepository::new(pool().await);
        let error = repo.get(42).await.expect_err("no such track");
        assert!(error.is_not_found());
        assert_eq!(error.code(), "NOT_FOUND");
    }

    #[tokio::test]
    async fn delete_missing_removes_tracks_fts_rows_and_orphans() {
        let db = pool().await;
        let repo = SqliteTrackRepository::new(db.clone());
        repo.upsert_many(&[
            full("content://a", "Alpha", "Kept Artist", "Kept Album"),
            full("content://b", "Bravo", "Gone Artist", "Gone Album"),
        ])
        .await
        .expect("scan");

        let removed = repo
            .delete_missing(&["content://a".to_owned()])
            .await
            .expect("cleanup");
        assert_eq!(removed, 1);

        let stats = repo.stats().await.expect("stats");
        assert_eq!(stats.tracks, 1);
        assert_eq!(stats.albums, 1, "the orphaned album must be gone");
        assert_eq!(stats.artists, 1, "the orphaned artist must be gone");
        assert_eq!(stats.genres, 1);

        let fts: i64 = sqlx::query("SELECT COUNT(*) AS n FROM tracks_fts")
            .fetch_one(&db)
            .await
            .expect("fts counted")
            .get("n");
        assert_eq!(fts, 1);

        let matched: i64 =
            sqlx::query("SELECT COUNT(*) AS n FROM tracks_fts WHERE tracks_fts MATCH '\"bravo\"*'")
                .fetch_one(&db)
                .await
                .expect("fts searched")
                .get("n");
        assert_eq!(matched, 0, "the deleted track must leave the index");
    }

    #[tokio::test]
    async fn stats_sum_the_library() {
        let db = pool().await;
        let repo = SqliteTrackRepository::new(db.clone());
        let mut a = full("content://a", "Alpha", "A", "A");
        a.duration_ms = 1_000;
        a.size = 10;
        let mut b = full("content://b", "Bravo", "B", "B");
        b.duration_ms = 2_000;
        b.size = 20;
        b.genre = Some("Jazz".to_owned());
        repo.upsert_many(&[a, b]).await.expect("scan");

        let stats = repo.stats().await.expect("stats");
        assert_eq!(stats.tracks, 2);
        assert_eq!(stats.albums, 2);
        assert_eq!(stats.artists, 2);
        assert_eq!(stats.playlists, 0);
        assert_eq!(stats.genres, 2);
        assert_eq!(stats.total_duration_ms, 3_000);
        assert_eq!(stats.total_size, 30);
    }

    #[tokio::test]
    async fn recently_added_is_newest_first() {
        let repo = SqliteTrackRepository::new(pool().await);
        let mut old = scanned("content://old", "Old");
        old.date_added = 1;
        let mut new = scanned("content://new", "New");
        new.date_added = 2;
        repo.upsert_many(&[old, new]).await.expect("scan");

        let recent = repo.recently_added(1).await.expect("recent");
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].title, "New");
    }
}
