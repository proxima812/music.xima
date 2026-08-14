//! `TrackRepository` on SQLite. Owns the scanner write path, including the
//! `tracks_fts` bookkeeping (CONTRACTS §3, §4).

use sqlx::sqlite::SqliteConnection;
use sqlx::{QueryBuilder, Row, Sqlite};

use crate::domain::{
    clamp_limit, normalize_sort_key, HiddenTrack, LibraryStats, Page, PendingDeletion,
    ScannedTrack, Track, TrackQuery,
};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::TrackRepository;

use super::pool::Db;
use super::sql::{
    conjunction, dyn_query, fts_delete, fts_insert, fts_payload, track_from_row, track_order_by,
    track_select, tracks_by_ids, BIND_CHUNK, TRACK_IS_VISIBLE, TRACK_JOINS,
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

    builder
        .push(conjunction(&mut has_where))
        .push(TRACK_IS_VISIBLE);

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

async fn delete_orphans(conn: &mut SqliteConnection) -> CoreResult<()> {
    sqlx::query(
        "DELETE FROM albums WHERE id NOT IN \
         (SELECT album_id FROM tracks WHERE album_id IS NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        "DELETE FROM artists WHERE id NOT IN \
           (SELECT artist_id FROM tracks WHERE artist_id IS NOT NULL) \
         AND id NOT IN \
           (SELECT artist_id FROM albums WHERE artist_id IS NOT NULL)",
    )
    .execute(&mut *conn)
    .await?;
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
        let row = dyn_query(format!(
            "{} WHERE t.id = ? AND {TRACK_IS_VISIBLE}",
            track_select()
        ))
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
            "{} WHERE {TRACK_IS_VISIBLE} ORDER BY t.date_added DESC, t.id DESC LIMIT ?",
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

    async fn hide(&self, track_id: i64, hidden_at: i64) -> CoreResult<()> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "INSERT INTO hidden_tracks (track_id, uri, hidden_at) \
             SELECT id, uri, ? FROM tracks WHERE id = ? \
             ON CONFLICT(track_id) DO NOTHING",
        )
        .bind(hidden_at)
        .bind(track_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            super::sql::ensure_track(&mut tx, track_id).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn restore(&self, track_id: i64) -> CoreResult<()> {
        let mut tx = self.pool.begin().await?;
        super::sql::ensure_track(&mut tx, track_id).await?;
        sqlx::query("DELETE FROM hidden_tracks WHERE track_id = ?")
            .bind(track_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn hidden(&self) -> CoreResult<Vec<HiddenTrack>> {
        let rows = dyn_query(format!(
            "SELECT {}, hidden.hidden_at AS hidden_at \
             FROM tracks t {} \
             JOIN hidden_tracks hidden ON hidden.track_id = t.id \
             ORDER BY hidden.hidden_at DESC, t.id DESC",
            super::sql::TRACK_COLUMNS,
            TRACK_JOINS
        ))
        .fetch_all(&self.pool)
        .await?;
        let mut hidden = Vec::with_capacity(rows.len());
        for row in &rows {
            hidden.push(HiddenTrack {
                track: track_from_row(row)?,
                hidden_at: row.try_get("hidden_at")?,
            });
        }
        Ok(hidden)
    }

    async fn begin_deletion(
        &self,
        track_id: i64,
        requested_at: i64,
    ) -> CoreResult<PendingDeletion> {
        let mut tx = self.pool.begin().await?;
        super::sql::ensure_track(&mut tx, track_id).await?;
        sqlx::query(
            "INSERT INTO pending_deletions (track_id, uri, requested_at) \
             SELECT id, uri, ? FROM tracks WHERE id = ? \
             ON CONFLICT(track_id) DO NOTHING",
        )
        .bind(requested_at)
        .bind(track_id)
        .execute(&mut *tx)
        .await?;
        let row = sqlx::query(
            "SELECT track_id, uri, requested_at, file_deleted \
             FROM pending_deletions WHERE track_id = ?",
        )
        .bind(track_id)
        .fetch_one(&mut *tx)
        .await?;
        let pending = PendingDeletion {
            track_id: row.try_get("track_id")?,
            uri: row.try_get("uri")?,
            requested_at: row.try_get("requested_at")?,
            file_deleted: row.try_get::<i64, _>("file_deleted")? != 0,
        };
        tx.commit().await?;
        Ok(pending)
    }

    async fn cancel_deletion(&self, track_id: i64) -> CoreResult<()> {
        let result = sqlx::query("DELETE FROM pending_deletions WHERE track_id = ?")
            .bind(track_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(CoreError::not_found("pending deletion", track_id));
        }
        Ok(())
    }

    async fn mark_file_deleted(&self, track_id: i64) -> CoreResult<()> {
        let result =
            sqlx::query("UPDATE pending_deletions SET file_deleted = 1 WHERE track_id = ?")
                .bind(track_id)
                .execute(&self.pool)
                .await?;
        if result.rows_affected() == 0 {
            return Err(CoreError::not_found("pending deletion", track_id));
        }
        Ok(())
    }

    async fn finalize_deletion(&self, track_id: i64) -> CoreResult<()> {
        let mut tx = self.pool.begin().await?;
        deindex_track(&mut tx, track_id).await?;
        let result = sqlx::query("DELETE FROM tracks WHERE id = ?")
            .bind(track_id)
            .execute(&mut *tx)
            .await?;
        if result.rows_affected() == 0 {
            return Err(CoreError::not_found("track", track_id));
        }
        delete_orphans(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn pending_deletions(&self) -> CoreResult<Vec<PendingDeletion>> {
        let rows = sqlx::query(
            "SELECT track_id, uri, requested_at, file_deleted FROM pending_deletions \
             ORDER BY requested_at ASC, track_id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut pending = Vec::with_capacity(rows.len());
        for row in &rows {
            pending.push(PendingDeletion {
                track_id: row.try_get("track_id")?,
                uri: row.try_get("uri")?,
                requested_at: row.try_get("requested_at")?,
                file_deleted: row.try_get::<i64, _>("file_deleted")? != 0,
            });
        }
        Ok(pending)
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

            let hidden = sqlx::query("SELECT 1 FROM hidden_tracks WHERE uri = ?")
                .bind(uri)
                .fetch_optional(&mut *tx)
                .await?;
            if hidden.is_some() {
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

        delete_orphans(&mut tx).await?;

        sqlx::query("DROP TABLE IF EXISTS scan_keep_uris")
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(removed)
    }

    async fn stats(&self) -> CoreResult<LibraryStats> {
        let row = sqlx::query(
            "SELECT \
               (SELECT COUNT(*) FROM tracks t WHERE NOT EXISTS \
                  (SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id)) AS tracks, \
               (SELECT COUNT(*) FROM albums al WHERE EXISTS \
                  (SELECT 1 FROM tracks t WHERE t.album_id = al.id AND NOT EXISTS \
                    (SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id))) AS albums, \
               (SELECT COUNT(*) FROM artists ar WHERE EXISTS \
                  (SELECT 1 FROM tracks t WHERE t.artist_id = ar.id AND NOT EXISTS \
                    (SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id))) AS artists, \
               (SELECT COUNT(*) FROM playlists) AS playlists, \
               (SELECT COUNT(DISTINCT genre) FROM tracks t \
                  WHERE genre IS NOT NULL AND TRIM(genre) <> '' AND NOT EXISTS \
                    (SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id)) AS genres, \
               (SELECT COALESCE(SUM(duration_ms), 0) FROM tracks t WHERE NOT EXISTS \
                  (SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id)) AS total_duration_ms, \
               (SELECT COALESCE(SUM(size), 0) FROM tracks t WHERE NOT EXISTS \
                  (SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id)) AS total_size",
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

    async fn inserted_track(db: &super::super::pool::Db, uri: &str) -> i64 {
        let repo = SqliteTrackRepository::new(db.clone());
        repo.upsert_many(&[scanned(uri, "Removal fixture")])
            .await
            .expect("track inserted");
        sqlx::query("SELECT id FROM tracks WHERE uri = ?")
            .bind(uri)
            .fetch_one(db)
            .await
            .expect("track row")
            .get("id")
    }

    #[tokio::test]
    async fn hidden_track_tombstone_is_unique_and_idempotent() {
        let db = pool().await;
        let track_id = inserted_track(&db, "content://hidden").await;

        for hidden_at in [100_i64, 200] {
            sqlx::query(
                "INSERT INTO hidden_tracks (track_id, uri, hidden_at) \
                 SELECT id, uri, ? FROM tracks WHERE id = ? \
                 ON CONFLICT(track_id) DO NOTHING",
            )
            .bind(hidden_at)
            .bind(track_id)
            .execute(&db)
            .await
            .expect("hide track");
        }

        let row = sqlx::query(
            "SELECT COUNT(*) AS count, MIN(track_id) AS track_id, \
                    MIN(uri) AS uri, MIN(hidden_at) AS hidden_at \
             FROM hidden_tracks",
        )
        .fetch_one(&db)
        .await
        .expect("hidden row");
        assert_eq!(row.get::<i64, _>("count"), 1);
        assert_eq!(row.get::<i64, _>("track_id"), track_id);
        assert_eq!(row.get::<String, _>("uri"), "content://hidden");
        assert_eq!(row.get::<i64, _>("hidden_at"), 100);
    }

    #[tokio::test]
    async fn pending_deletion_is_unique_per_track_and_uri() {
        let db = pool().await;
        let track_id = inserted_track(&db, "content://pending").await;
        let other_track_id = inserted_track(&db, "content://other").await;

        sqlx::query(
            "INSERT INTO pending_deletions (track_id, uri, requested_at) \
             SELECT id, uri, 100 FROM tracks WHERE id = ?",
        )
        .bind(track_id)
        .execute(&db)
        .await
        .expect("pending deletion");

        let duplicate_track = sqlx::query(
            "INSERT INTO pending_deletions (track_id, uri, requested_at) \
             VALUES (?, 'content://different', 200)",
        )
        .bind(track_id)
        .execute(&db)
        .await;
        assert!(duplicate_track.is_err(), "track id must be unique");

        let duplicate_uri = sqlx::query(
            "INSERT INTO pending_deletions (track_id, uri, requested_at) \
             VALUES (?, 'content://pending', 200)",
        )
        .bind(other_track_id)
        .execute(&db)
        .await;
        assert!(duplicate_uri.is_err(), "uri must be unique");
    }

    #[tokio::test]
    async fn deleting_a_track_cascades_removal_state() {
        let db = pool().await;
        let track_id = inserted_track(&db, "content://cascade").await;
        sqlx::query(
            "INSERT INTO hidden_tracks (track_id, uri, hidden_at) \
             SELECT id, uri, 100 FROM tracks WHERE id = ?",
        )
        .bind(track_id)
        .execute(&db)
        .await
        .expect("hidden row");
        sqlx::query(
            "INSERT INTO pending_deletions (track_id, uri, requested_at) \
             SELECT id, uri, 100 FROM tracks WHERE id = ?",
        )
        .bind(track_id)
        .execute(&db)
        .await
        .expect("pending row");

        sqlx::query("DELETE FROM tracks WHERE id = ?")
            .bind(track_id)
            .execute(&db)
            .await
            .expect("track deleted");

        let hidden: i64 = sqlx::query("SELECT COUNT(*) AS count FROM hidden_tracks")
            .fetch_one(&db)
            .await
            .expect("hidden state counted")
            .get("count");
        let pending: i64 = sqlx::query("SELECT COUNT(*) AS count FROM pending_deletions")
            .fetch_one(&db)
            .await
            .expect("pending state counted")
            .get("count");
        assert_eq!(hidden, 0, "hidden state must cascade");
        assert_eq!(pending, 0, "pending state must cascade");
    }

    #[tokio::test]
    async fn hide_lists_full_tracks_newest_first_and_restore_keeps_relationships() {
        let db = pool().await;
        let repo = SqliteTrackRepository::new(db.clone());
        repo.upsert_many(&[
            full("content://old-hidden", "Old hidden", "Artist", "Album"),
            full("content://new-hidden", "New hidden", "Artist", "Album"),
        ])
        .await
        .expect("scan");
        let ids: Vec<i64> = sqlx::query("SELECT id FROM tracks ORDER BY id")
            .fetch_all(&db)
            .await
            .expect("ids")
            .iter()
            .map(|row| row.get("id"))
            .collect();
        sqlx::query("INSERT INTO favorites (track_id, created_at) VALUES (?, 1)")
            .bind(ids[0])
            .execute(&db)
            .await
            .expect("favorite");

        repo.hide(ids[0], 100).await.expect("hide old");
        repo.hide(ids[1], 200).await.expect("hide new");
        repo.hide(ids[0], 999).await.expect("idempotent hide");
        let hidden = repo.hidden().await.expect("hidden tracks");
        assert_eq!(
            hidden.iter().map(|item| item.track.id).collect::<Vec<_>>(),
            vec![ids[1], ids[0]]
        );
        assert_eq!(hidden[1].hidden_at, 100);
        assert_eq!(hidden[1].track.uri, "content://old-hidden");
        assert!(hidden[1].track.is_favorite);

        repo.restore(ids[0]).await.expect("restore");
        assert_eq!(repo.hidden().await.expect("hidden tracks").len(), 1);
        let favorite_count: i64 =
            sqlx::query("SELECT COUNT(*) AS count FROM favorites WHERE track_id = ?")
                .bind(ids[0])
                .fetch_one(&db)
                .await
                .expect("favorite counted")
                .get("count");
        assert_eq!(favorite_count, 1);
        assert_eq!(
            repo.get(ids[0]).await.expect("restored track").title,
            "Old hidden"
        );

        let error = repo.hide(99_999, 1).await.expect_err("unknown track");
        assert_eq!(error.code(), "NOT_FOUND");
    }

    #[tokio::test]
    async fn pending_deletion_flow_persists_recovery_state_and_can_cancel() {
        let db = pool().await;
        let repo = SqliteTrackRepository::new(db.clone());
        let track_id = inserted_track(&db, "content://delete-later").await;
        sqlx::query("INSERT INTO favorites (track_id, created_at) VALUES (?, 1)")
            .bind(track_id)
            .execute(&db)
            .await
            .expect("favorite");

        let pending = repo.begin_deletion(track_id, 123).await.expect("begin");
        assert_eq!(pending.track_id, track_id);
        assert_eq!(pending.uri, "content://delete-later");
        assert_eq!(pending.requested_at, 123);
        assert!(!pending.file_deleted);
        repo.mark_file_deleted(track_id)
            .await
            .expect("mark deleted");
        assert!(repo.pending_deletions().await.expect("pending")[0].file_deleted);

        repo.cancel_deletion(track_id).await.expect("cancel");
        assert!(repo.pending_deletions().await.expect("pending").is_empty());
        assert_eq!(
            repo.get(track_id).await.expect("track survives").id,
            track_id
        );
        let favorites: i64 = sqlx::query("SELECT COUNT(*) AS count FROM favorites")
            .fetch_one(&db)
            .await
            .expect("favorites counted")
            .get("count");
        assert_eq!(favorites, 1);
        assert_eq!(
            repo.begin_deletion(99_999, 1)
                .await
                .expect_err("unknown track")
                .code(),
            "NOT_FOUND"
        );
        assert_eq!(
            repo.mark_file_deleted(99_999)
                .await
                .expect_err("unknown pending deletion")
                .code(),
            "NOT_FOUND"
        );
    }

    #[tokio::test]
    async fn finalize_deletion_removes_track_index_and_dependent_rows() {
        let db = pool().await;
        let repo = SqliteTrackRepository::new(db.clone());
        let track_id = inserted_track(&db, "content://delete-now").await;
        sqlx::query("INSERT INTO favorites (track_id, created_at) VALUES (?, 1)")
            .bind(track_id)
            .execute(&db)
            .await
            .expect("favorite");
        repo.begin_deletion(track_id, 123).await.expect("begin");
        repo.finalize_deletion(track_id).await.expect("finalize");

        assert!(repo
            .get(track_id)
            .await
            .expect_err("deleted")
            .is_not_found());
        let fts: i64 = sqlx::query("SELECT COUNT(*) AS count FROM tracks_fts WHERE rowid = ?")
            .bind(track_id)
            .fetch_one(&db)
            .await
            .expect("fts counted")
            .get("count");
        let favorites: i64 = sqlx::query("SELECT COUNT(*) AS count FROM favorites")
            .fetch_one(&db)
            .await
            .expect("favorites counted")
            .get("count");
        assert_eq!(fts, 0);
        assert_eq!(favorites, 0);
        assert_eq!(
            repo.finalize_deletion(track_id)
                .await
                .expect_err("already deleted")
                .code(),
            "NOT_FOUND"
        );
    }

    #[tokio::test]
    async fn scan_does_not_refresh_a_hidden_uri() {
        let db = pool().await;
        let repo = SqliteTrackRepository::new(db.clone());
        let track_id = inserted_track(&db, "content://tombstoned").await;
        repo.hide(track_id, 100).await.expect("hide");

        let mut changed = scanned("content://tombstoned", "Changed title");
        changed.last_modified = 999;
        assert_eq!(repo.upsert_many(&[changed]).await.expect("scan"), 0);

        let hidden = repo.hidden().await.expect("hidden");
        assert_eq!(hidden[0].track.title, "Removal fixture");
        assert!(repo
            .query(&TrackQuery::default())
            .await
            .expect("visible")
            .items
            .is_empty());
    }

    #[tokio::test]
    async fn hidden_tracks_are_excluded_from_every_visible_repository_surface() {
        use crate::domain::{SmartRule, SmartSort, StatsRange};
        use crate::infrastructure::repositories::{
            AlbumRepository, ArtistRepository, FavoriteRepository, HistoryRepository,
            PlaylistRepository, SearchRepository, SmartPlaylistRepository, StatisticsRepository,
            TaxonomyRepository,
        };
        use crate::infrastructure::sqlite::{
            SqliteAlbumRepository, SqliteArtistRepository, SqliteFavoriteRepository,
            SqliteHistoryRepository, SqlitePlaylistRepository, SqliteSearchRepository,
            SqliteSmartPlaylistRepository, SqliteStatisticsRepository, SqliteTaxonomyRepository,
        };

        let db = pool().await;
        let tracks = SqliteTrackRepository::new(db.clone());
        let mut hidden_scan = full(
            "content://hidden-surface",
            "Hidden Surface",
            "Hidden Artist",
            "Hidden Album",
        );
        hidden_scan.genre = Some("Hidden Genre".to_owned());
        hidden_scan.folder = Some("Hidden Folder".to_owned());
        let mut visible_scan = full(
            "content://visible-surface",
            "Visible Surface",
            "Visible Artist",
            "Visible Album",
        );
        visible_scan.genre = Some("Visible Genre".to_owned());
        visible_scan.folder = Some("Visible Folder".to_owned());
        tracks
            .upsert_many(&[hidden_scan, visible_scan])
            .await
            .expect("scan");
        let hidden_id: i64 = sqlx::query("SELECT id FROM tracks WHERE uri = ?")
            .bind("content://hidden-surface")
            .fetch_one(&db)
            .await
            .expect("hidden id")
            .get("id");
        let visible_id: i64 = sqlx::query("SELECT id FROM tracks WHERE uri = ?")
            .bind("content://visible-surface")
            .fetch_one(&db)
            .await
            .expect("visible id")
            .get("id");

        let favorites = SqliteFavoriteRepository::new(db.clone());
        favorites.toggle(hidden_id).await.expect("favorite hidden");
        favorites
            .toggle(visible_id)
            .await
            .expect("favorite visible");
        let history = SqliteHistoryRepository::new(db.clone());
        history
            .record(hidden_id, 100, 60_000)
            .await
            .expect("history hidden");
        history
            .record(visible_id, 200, 60_000)
            .await
            .expect("history visible");
        let playlists = SqlitePlaylistRepository::new(db.clone());
        let playlist = playlists.create("Visibility").await.expect("playlist");
        playlists
            .add_tracks(playlist.id, &[hidden_id, visible_id])
            .await
            .expect("playlist tracks");
        tracks.hide(hidden_id, 300).await.expect("hide");

        assert_eq!(
            tracks
                .query(&TrackQuery::default())
                .await
                .expect("library")
                .items
                .len(),
            1
        );
        assert_eq!(tracks.recently_added(10).await.expect("recent").len(), 1);
        assert_eq!(
            tracks
                .get_many(&[hidden_id, visible_id])
                .await
                .expect("many")
                .len(),
            1
        );
        assert_eq!(favorites.list().await.expect("favorites").len(), 1);
        assert_eq!(history.recent(10).await.expect("history").len(), 1);
        assert_eq!(
            playlists.tracks(playlist.id).await.expect("tracks").len(),
            1
        );
        assert_eq!(
            playlists
                .get(playlist.id)
                .await
                .expect("playlist")
                .track_count,
            1
        );
        let relation_count: i64 = sqlx::query(
            "SELECT (SELECT COUNT(*) FROM favorites) + \
                    (SELECT COUNT(*) FROM playlist_tracks) + \
                    (SELECT COUNT(*) FROM history) AS count",
        )
        .fetch_one(&db)
        .await
        .expect("relations")
        .get("count");
        assert_eq!(relation_count, 6, "hiding must preserve relationships");

        let albums = SqliteAlbumRepository::new(db.clone());
        let album_page = albums.query(0, 10).await.expect("albums");
        assert_eq!(album_page.total, 1);
        assert_eq!(album_page.items[0].track_count, 1);
        let artists = SqliteArtistRepository::new(db.clone());
        let artist_page = artists.query(0, 10).await.expect("artists");
        assert_eq!(artist_page.total, 1);
        assert_eq!(artist_page.items[0].track_count, 1);
        let search = SqliteSearchRepository::new(db.clone());
        let hidden_search = search.all("Hidden", 10).await.expect("search");
        assert!(hidden_search.tracks.is_empty());
        assert!(hidden_search.albums.is_empty());
        assert!(hidden_search.artists.is_empty());
        let smart = SqliteSmartPlaylistRepository::new(db.clone());
        assert!(smart
            .resolve(
                &[SmartRule::TitleContains {
                    value: "Hidden Surface".to_owned(),
                }],
                true,
                SmartSort::TitleAsc,
                None,
            )
            .await
            .expect("smart")
            .is_empty());
        let stats = SqliteStatisticsRepository::new(db.clone());
        assert_eq!(
            stats
                .top_tracks(StatsRange::AllTime, 10)
                .await
                .expect("top")
                .len(),
            1
        );
        let taxonomy = SqliteTaxonomyRepository::new(db);
        assert_eq!(taxonomy.genres().await.expect("genres").len(), 1);
        assert_eq!(taxonomy.folders(None).await.expect("folders").len(), 1);
    }

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
