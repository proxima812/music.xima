//! `HistoryRepository` on SQLite (CONTRACTS §3).
//!
//! `history` is the append-only log every statistic is derived from;
//! `play_counts` is the denormalised counter the library screens read. Both are
//! written in one transaction so they can never drift apart.

use crate::domain::{clamp_limit, Track};
use crate::error::CoreResult;
use crate::infrastructure::repositories::HistoryRepository;

use super::pool::Db;
use super::sql::{dyn_query, ensure_track, tracks_from_rows, TRACK_COLUMNS, TRACK_JOINS};

pub struct SqliteHistoryRepository {
    pool: Db,
}

impl SqliteHistoryRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }
}

/// A late-arriving row must never drag `last_played_at` backwards, hence the
/// `MAX` instead of a plain assignment.
const BUMP_PLAY_COUNT: &str = "INSERT INTO play_counts (track_id, count, last_played_at) \
     VALUES (?, 1, ?) \
     ON CONFLICT(track_id) DO UPDATE SET \
       count = count + 1, \
       last_played_at = MAX(COALESCE(last_played_at, 0), excluded.last_played_at)";

#[async_trait::async_trait]
impl HistoryRepository for SqliteHistoryRepository {
    async fn record(
        &self,
        track_id: i64,
        played_at: i64,
        duration_played_ms: i64,
    ) -> CoreResult<()> {
        let mut tx = self.pool.begin().await?;
        ensure_track(&mut tx, track_id).await?;

        sqlx::query(
            "INSERT INTO history (track_id, played_at, duration_played_ms) VALUES (?, ?, ?)",
        )
        .bind(track_id)
        .bind(played_at)
        .bind(duration_played_ms)
        .execute(&mut *tx)
        .await?;

        sqlx::query(BUMP_PLAY_COUNT)
            .bind(track_id)
            .bind(played_at)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Recently played tracks, most recent first, one row per track.
    async fn recent(&self, limit: i64) -> CoreResult<Vec<Track>> {
        let rows = dyn_query(format!(
            "SELECT {TRACK_COLUMNS}, MAX(h.played_at) AS played_at \
             FROM history h JOIN tracks t ON t.id = h.track_id {TRACK_JOINS} \
             GROUP BY t.id ORDER BY played_at DESC LIMIT ?"
        ))
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;
        Ok(tracks_from_rows(&rows)?)
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteHistoryRepository;
    use crate::infrastructure::repositories::{HistoryRepository, TrackRepository};
    use crate::infrastructure::sqlite::pool::Db;
    use crate::infrastructure::sqlite::sql::test_support::{full, pool};
    use crate::infrastructure::sqlite::track_repo::SqliteTrackRepository;
    use sqlx::Row;

    const NOW: i64 = 1_700_000_000_000;

    async fn seeded() -> (Db, SqliteHistoryRepository, SqliteTrackRepository, Vec<i64>) {
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

        (db.clone(), SqliteHistoryRepository::new(db), tracks, ids)
    }

    async fn play_count(db: &Db, track_id: i64) -> (i64, Option<i64>) {
        let row = sqlx::query("SELECT count, last_played_at FROM play_counts WHERE track_id = ?")
            .bind(track_id)
            .fetch_one(db)
            .await
            .expect("play_counts row");
        (
            row.get::<i64, _>("count"),
            row.get::<Option<i64>, _>("last_played_at"),
        )
    }

    #[tokio::test]
    async fn recording_writes_history_and_bumps_the_counter() {
        let (db, history, tracks, ids) = seeded().await;

        history.record(ids[0], NOW, 200_000).await.expect("first");
        history
            .record(ids[0], NOW + 1_000, 30_000)
            .await
            .expect("second");

        assert_eq!(play_count(&db, ids[0]).await, (2, Some(NOW + 1_000)));

        let rows: i64 = sqlx::query("SELECT COUNT(*) AS n FROM history WHERE track_id = ?")
            .bind(ids[0])
            .fetch_one(&db)
            .await
            .expect("history rows")
            .get("n");
        assert_eq!(rows, 2);

        let track = tracks.get(ids[0]).await.expect("track");
        assert_eq!(track.play_count, 2);
        assert_eq!(track.last_played_at, Some(NOW + 1_000));
    }

    #[tokio::test]
    async fn a_late_row_does_not_move_the_last_play_backwards() {
        let (db, history, _tracks, ids) = seeded().await;

        history.record(ids[0], NOW, 200_000).await.expect("now");
        history
            .record(ids[0], NOW - 60_000, 200_000)
            .await
            .expect("late");

        assert_eq!(play_count(&db, ids[0]).await, (2, Some(NOW)));
    }

    #[tokio::test]
    async fn recent_is_deduplicated_and_newest_first() {
        let (_db, history, _tracks, ids) = seeded().await;

        history.record(ids[0], NOW, 200_000).await.expect("alpha");
        history
            .record(ids[1], NOW + 1_000, 200_000)
            .await
            .expect("bravo");
        history
            .record(ids[0], NOW + 2_000, 200_000)
            .await
            .expect("alpha again");

        let recent = history.recent(10).await.expect("recent");
        assert_eq!(
            recent.iter().map(|track| track.id).collect::<Vec<_>>(),
            vec![ids[0], ids[1]]
        );
        assert_eq!(recent[0].play_count, 2);

        let capped = history.recent(1).await.expect("capped");
        assert_eq!(capped.len(), 1);
        assert_eq!(capped[0].id, ids[0]);
    }

    #[tokio::test]
    async fn unknown_track_is_not_recorded() {
        let (db, history, _tracks, _ids) = seeded().await;

        let error = history
            .record(4_242, NOW, 1_000)
            .await
            .expect_err("no such track");
        assert!(error.is_not_found());

        let rows: i64 = sqlx::query("SELECT COUNT(*) AS n FROM history")
            .fetch_one(&db)
            .await
            .expect("history rows")
            .get("n");
        assert_eq!(rows, 0, "the transaction must roll back");
    }
}
