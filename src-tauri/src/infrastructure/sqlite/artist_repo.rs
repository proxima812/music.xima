//! `ArtistRepository` on SQLite (CONTRACTS §3).

use sqlx::Row;

use crate::domain::{clamp_limit, clamp_offset, Artist, Page};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::ArtistRepository;

use super::pool::Db;
use super::sql::{artist_from_row, dyn_query, ARTIST_SELECT};

pub struct SqliteArtistRepository {
    pool: Db,
}

impl SqliteArtistRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ArtistRepository for SqliteArtistRepository {
    async fn get(&self, id: i64) -> CoreResult<Artist> {
        let row = dyn_query(format!(
            "{ARTIST_SELECT} WHERE ar.id = ? AND EXISTS (\
               SELECT 1 FROM tracks t WHERE t.artist_id = ar.id AND NOT EXISTS (\
                 SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id\
               )\
             )"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| CoreError::not_found("artist", id))?;
        Ok(artist_from_row(&row)?)
    }

    async fn query(&self, offset: i64, limit: i64) -> CoreResult<Page<Artist>> {
        let offset = clamp_offset(offset);
        let limit = clamp_limit(limit);

        let total: i64 = sqlx::query(
            "SELECT COUNT(*) AS total FROM artists ar WHERE EXISTS (\
               SELECT 1 FROM tracks t WHERE t.artist_id = ar.id AND NOT EXISTS (\
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
            "{ARTIST_SELECT} WHERE EXISTS (\
               SELECT 1 FROM tracks t WHERE t.artist_id = ar.id AND NOT EXISTS (\
                 SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id\
               )\
             ) ORDER BY ar.sort_name ASC, ar.id ASC LIMIT ? OFFSET ?"
        ))
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in &rows {
            items.push(artist_from_row(row)?);
        }
        Ok(Page::new(items, total, offset, limit))
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteArtistRepository;
    use crate::infrastructure::repositories::{ArtistRepository, TrackRepository};
    use crate::infrastructure::sqlite::sql::test_support::{full, pool};
    use crate::infrastructure::sqlite::track_repo::SqliteTrackRepository;

    async fn seeded() -> SqliteArtistRepository {
        let db = pool().await;
        let tracks = SqliteTrackRepository::new(db.clone());
        tracks
            .upsert_many(&[
                full("content://1", "One", "The Beatles", "Revolver"),
                full("content://2", "Two", "The Beatles", "Abbey Road"),
                full("content://3", "Three", "ABBA", "Arrival"),
            ])
            .await
            .expect("library scan");
        SqliteArtistRepository::new(db)
    }

    #[tokio::test]
    async fn artists_count_albums_and_tracks() {
        let artists = seeded().await;
        let page = artists.query(0, 10).await.expect("artists");

        assert_eq!(page.total, 2);
        // "The Beatles" is filed under "beatles", so it follows "abba".
        let names: Vec<&str> = page.items.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["ABBA", "The Beatles"]);

        let beatles = page.items.last().expect("artist");
        assert_eq!(beatles.album_count, 2);
        assert_eq!(beatles.track_count, 2);

        let fetched = artists.get(beatles.id).await.expect("artist by id");
        assert_eq!(&fetched, beatles);
    }

    #[tokio::test]
    async fn pagination_keeps_the_total() {
        let artists = seeded().await;
        let page = artists.query(0, 1).await.expect("first page");
        assert_eq!(page.total, 2);
        assert_eq!(page.items.len(), 1);
        assert!(page.has_more());

        let past_end = artists.query(50, 10).await.expect("past the end");
        assert!(past_end.items.is_empty());
        assert_eq!(past_end.total, 2);
    }

    #[tokio::test]
    async fn missing_artist_is_not_found() {
        let artists = seeded().await;
        assert!(artists
            .get(777)
            .await
            .expect_err("no artist")
            .is_not_found());
    }
}
