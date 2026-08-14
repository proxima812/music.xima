//! `AlbumRepository` on SQLite (CONTRACTS §3).

use sqlx::Row;

use crate::domain::{clamp_limit, clamp_offset, Album, Page};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::AlbumRepository;

use super::pool::Db;
use super::sql::{album_from_row, dyn_query, ALBUM_SELECT};

pub struct SqliteAlbumRepository {
    pool: Db,
}

impl SqliteAlbumRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl AlbumRepository for SqliteAlbumRepository {
    async fn get(&self, id: i64) -> CoreResult<Album> {
        let row = dyn_query(format!("{ALBUM_SELECT} WHERE al.id = ? GROUP BY al.id"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| CoreError::not_found("album", id))?;
        Ok(album_from_row(&row)?)
    }

    async fn query(&self, offset: i64, limit: i64) -> CoreResult<Page<Album>> {
        let offset = clamp_offset(offset);
        let limit = clamp_limit(limit);

        let total: i64 = sqlx::query(
            "SELECT COUNT(*) AS total FROM albums al WHERE EXISTS (\
               SELECT 1 FROM tracks t WHERE t.album_id = al.id AND NOT EXISTS (\
                 SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id\
               )\
             )",
        )
        .fetch_one(&self.pool)
        .await?
        .try_get("total")?;
        if total <= offset {
            return Ok(Page::new(Vec::new(), total, offset, limit));
        }

        let rows = dyn_query(format!(
            "{ALBUM_SELECT} GROUP BY al.id \
             ORDER BY al.sort_title ASC, al.id ASC LIMIT ? OFFSET ?"
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            items.push(album_from_row(row)?);
        }
        Ok(Page::new(items, total, offset, limit))
    }

    /// Albums credited to the artist plus the ones they only appear on, so a
    /// featured track never hides its album from the artist screen.
    async fn by_artist(&self, artist_id: i64) -> CoreResult<Vec<Album>> {
        let rows = dyn_query(format!(
            "{ALBUM_SELECT} \
             WHERE al.artist_id = ? \
                OR al.id IN (SELECT album_id FROM tracks \
                             WHERE artist_id = ? AND album_id IS NOT NULL \
                               AND NOT EXISTS (SELECT 1 FROM hidden_tracks hidden \
                                               WHERE hidden.track_id = tracks.id)) \
             GROUP BY al.id \
             ORDER BY al.year DESC, al.sort_title ASC, al.id ASC"
        ))
        .bind(artist_id)
        .bind(artist_id)
        .fetch_all(&self.pool)
        .await?;

        let mut albums = Vec::with_capacity(rows.len());
        for row in &rows {
            albums.push(album_from_row(row)?);
        }
        Ok(albums)
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteAlbumRepository;
    use crate::infrastructure::repositories::{AlbumRepository, TrackRepository};
    use crate::infrastructure::sqlite::sql::test_support::{full, pool};
    use crate::infrastructure::sqlite::track_repo::SqliteTrackRepository;

    async fn seeded() -> (SqliteAlbumRepository, SqliteTrackRepository) {
        let db = pool().await;
        let tracks = SqliteTrackRepository::new(db.clone());

        let mut one = full("content://1", "One", "Portishead", "Dummy");
        one.duration_ms = 1_000;
        one.track_number = Some(1);
        let mut two = full("content://2", "Two", "Portishead", "Dummy");
        two.duration_ms = 2_000;
        two.track_number = Some(2);
        let mut three = full("content://3", "Three", "Massive Attack", "Mezzanine");
        three.year = Some(1998);
        tracks
            .upsert_many(&[one, two, three])
            .await
            .expect("library scan");

        (SqliteAlbumRepository::new(db), tracks)
    }

    #[tokio::test]
    async fn albums_aggregate_their_tracks() {
        let (albums, _tracks) = seeded().await;

        let page = albums.query(0, 10).await.expect("albums");
        assert_eq!(page.total, 2);
        let titles: Vec<&str> = page.items.iter().map(|a| a.title.as_str()).collect();
        assert_eq!(titles, vec!["Dummy", "Mezzanine"]);

        let dummy = page.items.first().expect("first album");
        assert_eq!(dummy.track_count, 2);
        assert_eq!(dummy.duration_ms, 3_000);
        assert_eq!(dummy.artist_name.as_deref(), Some("Portishead"));

        let fetched = albums.get(dummy.id).await.expect("album by id");
        assert_eq!(&fetched, dummy);
    }

    #[tokio::test]
    async fn pagination_reports_the_full_total() {
        let (albums, _tracks) = seeded().await;

        let page = albums.query(1, 1).await.expect("second page");
        assert_eq!(page.total, 2);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].title, "Mezzanine");
        assert!(!page.has_more());

        let past_end = albums.query(9, 10).await.expect("past the end");
        assert!(past_end.items.is_empty());
        assert_eq!(past_end.total, 2);
    }

    #[tokio::test]
    async fn by_artist_lists_newest_first() {
        let (albums, _tracks) = seeded().await;
        let all = albums.query(0, 10).await.expect("albums");
        let mezzanine = all
            .items
            .iter()
            .find(|a| a.title == "Mezzanine")
            .expect("album");
        let artist_id = mezzanine.artist_id.expect("album has an artist");

        let by_artist = albums.by_artist(artist_id).await.expect("by artist");
        assert_eq!(by_artist.len(), 1);
        assert_eq!(by_artist[0].title, "Mezzanine");
        assert_eq!(by_artist[0].year, Some(1998));

        assert!(albums.by_artist(9_999).await.expect("unknown").is_empty());
    }

    #[tokio::test]
    async fn missing_album_is_not_found() {
        let (albums, _tracks) = seeded().await;
        let error = albums.get(4_242).await.expect_err("no such album");
        assert!(error.is_not_found());
    }
}
