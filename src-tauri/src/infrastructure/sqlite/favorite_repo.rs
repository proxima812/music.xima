//! `FavoriteRepository` on SQLite (CONTRACTS §3).

use crate::domain::Track;
use crate::error::CoreResult;
use crate::infrastructure::repositories::FavoriteRepository;

use super::pool::Db;
use super::sql::{
    dyn_query, ensure_track, now_ms, track_select, tracks_from_rows, TRACK_IS_VISIBLE,
};

pub struct SqliteFavoriteRepository {
    pool: Db,
}

impl SqliteFavoriteRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl FavoriteRepository for SqliteFavoriteRepository {
    /// Flips the flag inside one transaction and reports the state the frontend
    /// should now show.
    async fn toggle(&self, track_id: i64) -> CoreResult<bool> {
        let mut tx = self.pool.begin().await?;
        ensure_track(&mut tx, track_id).await?;

        let removed = sqlx::query("DELETE FROM favorites WHERE track_id = ?")
            .bind(track_id)
            .execute(&mut *tx)
            .await?;

        let is_favorite = if removed.rows_affected() == 0 {
            sqlx::query("INSERT INTO favorites (track_id, created_at) VALUES (?, ?)")
                .bind(track_id)
                .bind(now_ms())
                .execute(&mut *tx)
                .await?;
            true
        } else {
            false
        };

        tx.commit().await?;
        Ok(is_favorite)
    }

    /// Most recently favorited first.
    async fn list(&self) -> CoreResult<Vec<Track>> {
        let rows = dyn_query(format!(
            "{} WHERE f.track_id IS NOT NULL AND {TRACK_IS_VISIBLE} \
             ORDER BY f.created_at DESC, t.id DESC",
            track_select()
        ))
        .fetch_all(&self.pool)
        .await?;
        Ok(tracks_from_rows(&rows)?)
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteFavoriteRepository;
    use crate::infrastructure::repositories::{FavoriteRepository, TrackRepository};
    use crate::infrastructure::sqlite::sql::test_support::{full, pool};
    use crate::infrastructure::sqlite::track_repo::SqliteTrackRepository;

    async fn seeded() -> (SqliteFavoriteRepository, SqliteTrackRepository, Vec<i64>) {
        let db = pool().await;
        let tracks = SqliteTrackRepository::new(db.clone());
        tracks
            .upsert_many(&[
                full("content://a", "Alpha", "A", "A"),
                full("content://b", "Bravo", "B", "B"),
            ])
            .await
            .expect("library scan");

        let ids = tracks
            .query(&crate::domain::TrackQuery::default())
            .await
            .expect("tracks")
            .items
            .iter()
            .map(|track| track.id)
            .collect();

        (SqliteFavoriteRepository::new(db), tracks, ids)
    }

    #[tokio::test]
    async fn toggling_flips_the_flag_and_the_track_view() {
        let (favorites, tracks, ids) = seeded().await;

        assert!(favorites.toggle(ids[0]).await.expect("on"));
        assert!(tracks.get(ids[0]).await.expect("track").is_favorite);
        assert_eq!(favorites.list().await.expect("list").len(), 1);

        assert!(!favorites.toggle(ids[0]).await.expect("off"));
        assert!(!tracks.get(ids[0]).await.expect("track").is_favorite);
        assert!(favorites.list().await.expect("list").is_empty());
    }

    #[tokio::test]
    async fn favorites_are_newest_first() {
        let (favorites, _tracks, ids) = seeded().await;
        favorites.toggle(ids[0]).await.expect("first");
        favorites.toggle(ids[1]).await.expect("second");

        let listed = favorites.list().await.expect("list");
        assert_eq!(listed.len(), 2);
        assert!(listed.iter().all(|track| track.is_favorite));
        assert_eq!(listed[0].id, ids[1]);
    }

    #[tokio::test]
    async fn unknown_track_cannot_be_favorited() {
        let (favorites, _tracks, _ids) = seeded().await;
        let error = favorites.toggle(4_242).await.expect_err("no such track");
        assert!(error.is_not_found());
    }
}
