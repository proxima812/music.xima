//! `PlaylistRepository` on SQLite (CONTRACTS §3).
//!
//! `playlist_tracks` has `PRIMARY KEY (playlist_id, position)` and positions
//! stay dense (`0..n-1`). Every move therefore parks the rows it shifts in the
//! negative range first, where they cannot collide with the rows still holding
//! their old slot, and decodes them back afterwards.

use sqlx::sqlite::SqliteConnection;
use sqlx::Row;

use crate::domain::{reorder_target, sanitize_playlist_name, Playlist, Track};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::PlaylistRepository;

use super::pool::Db;
use super::sql::{
    dyn_query, ensure_playlist, now_ms, playlist_from_row, tracks_from_rows, PLAYLIST_SELECT,
    TRACK_COLUMNS, TRACK_IS_VISIBLE, TRACK_JOINS,
};

pub struct SqlitePlaylistRepository {
    pool: Db,
}

impl SqlitePlaylistRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }
}

fn playlist_tracks_sql() -> String {
    format!(
        "SELECT {TRACK_COLUMNS} FROM playlist_tracks pt \
         JOIN tracks t ON t.id = pt.track_id {TRACK_JOINS} \
         WHERE pt.playlist_id = ? AND {TRACK_IS_VISIBLE} ORDER BY pt.position ASC"
    )
}

async fn touch(conn: &mut SqliteConnection, id: i64) -> CoreResult<()> {
    sqlx::query("UPDATE playlists SET updated_at = ? WHERE id = ?")
        .bind(now_ms())
        .bind(id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

async fn visible_rows(conn: &mut SqliteConnection, id: i64) -> CoreResult<Vec<(i64, i64)>> {
    let rows = dyn_query(format!(
        "SELECT pt.position AS position, pt.track_id AS track_id \
         FROM playlist_tracks pt JOIN tracks t ON t.id = pt.track_id \
         WHERE pt.playlist_id = ? AND {TRACK_IS_VISIBLE} \
         ORDER BY pt.position ASC"
    ))
    .bind(id)
    .fetch_all(&mut *conn)
    .await?;
    rows.iter()
        .map(|row| Ok((row.try_get("position")?, row.try_get("track_id")?)))
        .collect()
}

/// Brings every parked row back into the dense range.
async fn unpark(conn: &mut SqliteConnection, id: i64) -> CoreResult<()> {
    sqlx::query(
        "UPDATE playlist_tracks SET position = -position - 2 \
         WHERE playlist_id = ? AND position <= -2",
    )
    .bind(id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

#[async_trait::async_trait]
impl PlaylistRepository for SqlitePlaylistRepository {
    async fn list(&self) -> CoreResult<Vec<Playlist>> {
        let rows = dyn_query(format!(
            "{PLAYLIST_SELECT} GROUP BY p.id ORDER BY p.name COLLATE NOCASE ASC, p.id ASC"
        ))
        .fetch_all(&self.pool)
        .await?;

        let mut playlists = Vec::with_capacity(rows.len());
        for row in &rows {
            playlists.push(playlist_from_row(row)?);
        }
        Ok(playlists)
    }

    async fn get(&self, id: i64) -> CoreResult<Playlist> {
        let row = dyn_query(format!("{PLAYLIST_SELECT} WHERE p.id = ? GROUP BY p.id"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| CoreError::not_found("playlist", id))?;
        Ok(playlist_from_row(&row)?)
    }

    async fn create(&self, name: &str) -> CoreResult<Playlist> {
        let name = sanitize_playlist_name(name)?;
        let now = now_ms();
        let created =
            sqlx::query("INSERT INTO playlists (name, created_at, updated_at) VALUES (?, ?, ?)")
                .bind(&name)
                .bind(now)
                .bind(now)
                .execute(&self.pool)
                .await?;
        self.get(created.last_insert_rowid()).await
    }

    async fn rename(&self, id: i64, name: &str) -> CoreResult<()> {
        let name = sanitize_playlist_name(name)?;
        let updated = sqlx::query("UPDATE playlists SET name = ?, updated_at = ? WHERE id = ?")
            .bind(&name)
            .bind(now_ms())
            .bind(id)
            .execute(&self.pool)
            .await?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::not_found("playlist", id));
        }
        Ok(())
    }

    async fn delete(&self, id: i64) -> CoreResult<()> {
        let deleted = sqlx::query("DELETE FROM playlists WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if deleted.rows_affected() == 0 {
            return Err(CoreError::not_found("playlist", id));
        }
        Ok(())
    }

    async fn tracks(&self, id: i64) -> CoreResult<Vec<Track>> {
        let mut conn = self.pool.acquire().await?;
        ensure_playlist(&mut conn, id).await?;
        let rows = dyn_query(playlist_tracks_sql())
            .bind(id)
            .fetch_all(&mut *conn)
            .await?;
        Ok(tracks_from_rows(&rows)?)
    }

    /// Appends to the end, duplicates allowed. Ids that no longer exist are
    /// skipped rather than failing the whole call: the caller usually holds a
    /// selection made before the last scan.
    async fn add_tracks(&self, id: i64, track_ids: &[i64]) -> CoreResult<()> {
        if track_ids.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;
        ensure_playlist(&mut tx, id).await?;

        let row = sqlx::query(
            "SELECT COALESCE(MAX(position), -1) AS last FROM playlist_tracks WHERE playlist_id = ?",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let mut next: i64 = row.try_get::<i64, _>("last")? + 1;

        for track_id in track_ids {
            let inserted = sqlx::query(
                "INSERT INTO playlist_tracks (playlist_id, track_id, position) \
                 SELECT ?, id, ? FROM tracks WHERE id = ?",
            )
            .bind(id)
            .bind(next)
            .bind(*track_id)
            .execute(&mut *tx)
            .await?;
            if inserted.rows_affected() > 0 {
                next += 1;
            }
        }

        touch(&mut tx, id).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn remove_at(&self, id: i64, position: i64) -> CoreResult<()> {
        let mut tx = self.pool.begin().await?;
        ensure_playlist(&mut tx, id).await?;

        let rows = visible_rows(&mut tx, id).await?;
        let raw_position = usize::try_from(position)
            .ok()
            .and_then(|ordinal| rows.get(ordinal))
            .map(|(raw_position, _)| *raw_position)
            .ok_or_else(|| CoreError::not_found("playlist position", position))?;

        let removed =
            sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id = ? AND position = ?")
                .bind(id)
                .bind(raw_position)
                .execute(&mut *tx)
                .await?;
        if removed.rows_affected() == 0 {
            return Err(CoreError::not_found("playlist position", position));
        }

        // Everything behind the hole moves one slot forward: park first
        // (`new + 2` encoded negative), then decode into the freed slots.
        sqlx::query(
            "UPDATE playlist_tracks SET position = -(position + 1) \
             WHERE playlist_id = ? AND position > ?",
        )
        .bind(id)
        .bind(raw_position)
        .execute(&mut *tx)
        .await?;
        unpark(&mut tx, id).await?;

        touch(&mut tx, id).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn reorder(&self, id: i64, from: i64, to: i64) -> CoreResult<()> {
        let mut tx = self.pool.begin().await?;
        ensure_playlist(&mut tx, id).await?;

        let rows = visible_rows(&mut tx, id).await?;
        let len = i64::try_from(rows.len()).unwrap_or(i64::MAX);
        if from < 0 || to < 0 || from >= len || to >= len {
            return Err(CoreError::invalid_input(format!(
                "cannot move {from} to {to} in a playlist of {len}"
            )));
        }
        let Some(target) = reorder_target(len, from, to) else {
            return Ok(());
        };

        let from_index = usize::try_from(from)
            .map_err(|_| CoreError::invalid_input("playlist position is too large"))?;
        let target_index = usize::try_from(target)
            .map_err(|_| CoreError::invalid_input("playlist position is too large"))?;
        let slots: Vec<i64> = rows.iter().map(|(raw_position, _)| *raw_position).collect();
        let mut track_ids: Vec<i64> = rows.iter().map(|(_, track_id)| *track_id).collect();
        let moved = track_ids.remove(from_index);
        track_ids.insert(target_index, moved);
        for (raw_position, track_id) in slots.iter().zip(&track_ids) {
            sqlx::query(
                "UPDATE playlist_tracks SET track_id = ? \
                 WHERE playlist_id = ? AND position = ?",
            )
            .bind(*track_id)
            .bind(id)
            .bind(*raw_position)
            .execute(&mut *tx)
            .await?;
        }

        touch(&mut tx, id).await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SqlitePlaylistRepository;
    use crate::domain::MAX_PLAYLIST_NAME_LEN;
    use crate::infrastructure::repositories::{PlaylistRepository, TrackRepository};
    use crate::infrastructure::sqlite::pool::Db;
    use crate::infrastructure::sqlite::sql::test_support::{full, pool};
    use crate::infrastructure::sqlite::track_repo::SqliteTrackRepository;
    use sqlx::Row;

    /// Four tracks in the library, one playlist holding all of them in order.
    async fn seeded() -> (Db, SqlitePlaylistRepository, i64, Vec<i64>) {
        let db = pool().await;
        let tracks = SqliteTrackRepository::new(db.clone());
        tracks
            .upsert_many(&[
                full("content://a", "Alpha", "A", "A"),
                full("content://b", "Bravo", "B", "B"),
                full("content://c", "Charlie", "C", "C"),
                full("content://d", "Delta", "D", "D"),
            ])
            .await
            .expect("library scan");

        let ids: Vec<i64> = tracks
            .query(&crate::domain::TrackQuery::default())
            .await
            .expect("tracks")
            .items
            .iter()
            .map(|track| track.id)
            .collect();

        let playlists = SqlitePlaylistRepository::new(db.clone());
        let playlist = playlists.create("  Road trip ").await.expect("created");
        playlists
            .add_tracks(playlist.id, &ids)
            .await
            .expect("filled");

        (db, playlists, playlist.id, ids)
    }

    async fn positions(db: &Db, playlist_id: i64) -> Vec<(i64, i64)> {
        sqlx::query(
            "SELECT position, track_id FROM playlist_tracks WHERE playlist_id = ? ORDER BY position",
        )
        .bind(playlist_id)
        .fetch_all(db)
        .await
        .expect("positions")
        .iter()
        .map(|row| (row.get::<i64, _>("position"), row.get::<i64, _>("track_id")))
            .collect()
    }

    async fn hide_track(db: &Db, track_id: i64) {
        SqliteTrackRepository::new(db.clone())
            .hide(track_id, 100)
            .await
            .expect("hidden");
    }

    fn dense(rows: &[(i64, i64)]) -> bool {
        rows.iter()
            .enumerate()
            .all(|(index, (position, _))| *position == index as i64)
    }

    #[tokio::test]
    async fn create_trims_the_name_and_aggregates_tracks() {
        let (_db, playlists, id, ids) = seeded().await;

        let playlist = playlists.get(id).await.expect("playlist");
        assert_eq!(playlist.name, "Road trip");
        assert_eq!(playlist.track_count, ids.len() as i64);
        assert_eq!(playlist.duration_ms, 180_000 * ids.len() as i64);
        assert!(playlist.created_at > 0);

        let listed = playlists.list().await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, id);

        assert!(playlists.create("   ").await.is_err());
        assert!(playlists
            .create(&"x".repeat(MAX_PLAYLIST_NAME_LEN + 1))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn empty_playlist_is_readable() {
        let (_db, playlists, _id, _ids) = seeded().await;
        let empty = playlists.create("Empty").await.expect("created");
        assert_eq!(empty.track_count, 0);
        assert_eq!(empty.duration_ms, 0);
        assert!(empty.cover_key.is_none());
        assert!(playlists.tracks(empty.id).await.expect("tracks").is_empty());
    }

    #[tokio::test]
    async fn add_tracks_appends_and_skips_unknown_ids() {
        let (db, playlists, id, ids) = seeded().await;

        playlists
            .add_tracks(id, &[9_999, ids[0]])
            .await
            .expect("appended");

        let rows = positions(&db, id).await;
        assert!(dense(&rows), "{rows:?} is not dense");
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[4].1, ids[0], "the duplicate lands at the end");
    }

    #[tokio::test]
    async fn tracks_come_back_in_playlist_order() {
        let (db, playlists, id, ids) = seeded().await;
        playlists.reorder(id, 0, 3).await.expect("reordered");

        let expected: Vec<i64> = positions(&db, id).await.iter().map(|(_, t)| *t).collect();
        let tracks = playlists.tracks(id).await.expect("tracks");
        assert_eq!(
            tracks.iter().map(|t| t.id).collect::<Vec<_>>(),
            expected,
            "expected {ids:?} reordered"
        );
    }

    #[tokio::test]
    async fn remove_at_keeps_positions_dense() {
        let (db, playlists, id, ids) = seeded().await;

        playlists.remove_at(id, 1).await.expect("removed");
        let rows = positions(&db, id).await;
        assert!(dense(&rows), "{rows:?} is not dense");
        assert_eq!(
            rows.iter().map(|(_, t)| *t).collect::<Vec<_>>(),
            vec![ids[0], ids[2], ids[3]]
        );

        playlists.remove_at(id, 2).await.expect("removed last");
        let rows = positions(&db, id).await;
        assert!(dense(&rows));
        assert_eq!(rows.len(), 2);

        assert!(playlists
            .remove_at(id, 9)
            .await
            .expect_err("no such position")
            .is_not_found());
    }

    #[tokio::test]
    async fn remove_at_resolves_visible_ordinals_around_hidden_rows() {
        for (hidden_index, remove_visible, expected_indexes) in [
            (0_usize, 1_i64, vec![0_usize, 1, 3]),
            (1, 1, vec![0, 1, 3]),
            (3, 1, vec![0, 2, 3]),
        ] {
            let (db, playlists, id, ids) = seeded().await;
            hide_track(&db, ids[hidden_index]).await;

            playlists
                .remove_at(id, remove_visible)
                .await
                .expect("visible row removed");

            let rows = positions(&db, id).await;
            assert!(dense(&rows), "raw positions must stay dense: {rows:?}");
            assert_eq!(
                rows.iter()
                    .map(|(_, track_id)| *track_id)
                    .collect::<Vec<_>>(),
                expected_indexes
                    .iter()
                    .map(|index| ids[*index])
                    .collect::<Vec<_>>()
            );
            assert!(
                rows.iter()
                    .any(|(_, track_id)| *track_id == ids[hidden_index]),
                "hidden relationship must remain"
            );
        }
    }

    #[tokio::test]
    async fn reorder_moves_forward_and_back() {
        let (db, playlists, id, ids) = seeded().await;

        playlists.reorder(id, 0, 2).await.expect("forward");
        let rows = positions(&db, id).await;
        assert!(dense(&rows), "{rows:?} is not dense");
        assert_eq!(
            rows.iter().map(|(_, t)| *t).collect::<Vec<_>>(),
            vec![ids[1], ids[2], ids[0], ids[3]]
        );

        playlists.reorder(id, 3, 0).await.expect("backward");
        let rows = positions(&db, id).await;
        assert!(dense(&rows), "{rows:?} is not dense");
        assert_eq!(
            rows.iter().map(|(_, t)| *t).collect::<Vec<_>>(),
            vec![ids[3], ids[1], ids[2], ids[0]]
        );
    }

    #[tokio::test]
    async fn reorder_remaps_visible_rows_without_moving_hidden_slots() {
        for (hidden_index, forward_indexes, backward_indexes) in [
            (0_usize, vec![2_usize, 3, 1], vec![1_usize, 2, 3]),
            (1, vec![2, 3, 0], vec![0, 2, 3]),
            (3, vec![1, 2, 0], vec![0, 1, 2]),
        ] {
            let (db, playlists, id, ids) = seeded().await;
            hide_track(&db, ids[hidden_index]).await;
            let hidden_position = positions(&db, id)
                .await
                .iter()
                .find(|(_, track_id)| *track_id == ids[hidden_index])
                .expect("hidden row")
                .0;

            playlists.reorder(id, 0, 2).await.expect("forward");
            let forward = positions(&db, id).await;
            assert_eq!(
                forward
                    .iter()
                    .find(|(_, track_id)| *track_id == ids[hidden_index])
                    .expect("hidden row")
                    .0,
                hidden_position,
                "hidden slot moved during forward reorder"
            );
            let visible_forward = playlists.tracks(id).await.expect("visible tracks");
            assert_eq!(
                visible_forward
                    .iter()
                    .map(|track| track.id)
                    .collect::<Vec<_>>(),
                forward_indexes
                    .iter()
                    .map(|index| ids[*index])
                    .collect::<Vec<_>>()
            );

            playlists.reorder(id, 2, 0).await.expect("backward");
            let backward = positions(&db, id).await;
            assert_eq!(
                backward
                    .iter()
                    .find(|(_, track_id)| *track_id == ids[hidden_index])
                    .expect("hidden row")
                    .0,
                hidden_position,
                "hidden slot moved during backward reorder"
            );
            assert_eq!(
                playlists
                    .tracks(id)
                    .await
                    .expect("visible tracks")
                    .iter()
                    .map(|track| track.id)
                    .collect::<Vec<_>>(),
                backward_indexes
                    .iter()
                    .map(|index| ids[*index])
                    .collect::<Vec<_>>()
            );
        }
    }

    #[tokio::test]
    async fn reorder_rejects_out_of_range_and_ignores_no_ops() {
        let (db, playlists, id, _ids) = seeded().await;
        let before = positions(&db, id).await;

        playlists.reorder(id, 2, 2).await.expect("no-op");
        assert_eq!(positions(&db, id).await, before);

        let error = playlists.reorder(id, 0, 4).await.expect_err("out of range");
        assert_eq!(error.code(), "INVALID_INPUT");
        assert_eq!(positions(&db, id).await, before);
    }

    #[tokio::test]
    async fn rename_and_delete_report_missing_playlists() {
        let (db, playlists, id, _ids) = seeded().await;

        playlists.rename(id, " Weekend ").await.expect("renamed");
        assert_eq!(playlists.get(id).await.expect("playlist").name, "Weekend");
        assert!(playlists.rename(id, "  ").await.is_err());
        assert!(playlists
            .rename(4_242, "Nope")
            .await
            .expect_err("missing")
            .is_not_found());

        playlists.delete(id).await.expect("deleted");
        assert!(playlists.get(id).await.expect_err("gone").is_not_found());
        assert!(playlists
            .delete(id)
            .await
            .expect_err("already gone")
            .is_not_found());
        assert!(positions(&db, id).await.is_empty(), "rows must cascade");

        assert!(playlists
            .tracks(id)
            .await
            .expect_err("missing playlist")
            .is_not_found());
        assert!(playlists
            .add_tracks(id, &[1])
            .await
            .expect_err("missing playlist")
            .is_not_found());
    }
}
