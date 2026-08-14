//! `StatisticsRepository` on SQLite (CONTRACTS §3).
//!
//! Everything here reads `history` inside the window of a [`StatsRange`],
//! counting only rows that qualify as a play: half a minute of listening, or
//! half the track.

use sqlx::{QueryBuilder, Row, Sqlite};

use crate::domain::{clamp_limit, RankedTrack, StatsRange, Track, MS_PER_DAY};
use crate::error::CoreResult;
use crate::infrastructure::repositories::StatisticsRepository;

use super::pool::Db;
use super::sql::{
    dyn_query, now_ms, track_from_row, track_select, tracks_from_rows, COUNTS_AS_PLAY,
    TRACK_COLUMNS, TRACK_JOINS,
};

pub struct SqliteStatisticsRepository {
    pool: Db,
}

impl SqliteStatisticsRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }
}

/// Narrows an aggregate to the requested window. `AllTime` adds nothing.
fn push_range(builder: &mut QueryBuilder<Sqlite>, range: StatsRange) {
    if let Some(since) = range.since_ms(now_ms()) {
        builder.push(" AND h.played_at >= ").push_bind(since);
    }
}

#[async_trait::async_trait]
impl StatisticsRepository for SqliteStatisticsRepository {
    async fn top_tracks(&self, range: StatsRange, limit: i64) -> CoreResult<Vec<RankedTrack>> {
        let mut builder = QueryBuilder::<Sqlite>::new(format!(
            "SELECT {TRACK_COLUMNS}, COUNT(*) AS plays, \
             COALESCE(SUM(h.duration_played_ms), 0) AS listening_time_ms \
             FROM history h JOIN tracks t ON t.id = h.track_id {TRACK_JOINS} \
             WHERE {COUNTS_AS_PLAY}"
        ));
        push_range(&mut builder, range);
        builder.push(
            " GROUP BY t.id ORDER BY plays DESC, listening_time_ms DESC, t.sort_title ASC LIMIT ",
        );
        builder.push_bind(clamp_limit(limit));

        let rows = builder.build().fetch_all(&self.pool).await?;
        let mut ranked = Vec::with_capacity(rows.len());
        for row in &rows {
            ranked.push(RankedTrack {
                track: track_from_row(row)?,
                plays: row.try_get("plays")?,
                listening_time_ms: row.try_get("listening_time_ms")?,
            });
        }
        Ok(ranked)
    }

    /// Never played means the counter is still at zero, the same thing the
    /// `NEVER_PLAYED` smart rule looks at.
    async fn never_played(&self, limit: i64) -> CoreResult<Vec<Track>> {
        let rows = dyn_query(format!(
            "{} WHERE COALESCE(pc.count, 0) = 0 ORDER BY t.date_added DESC, t.id DESC LIMIT ?",
            track_select()
        ))
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;
        Ok(tracks_from_rows(&rows)?)
    }

    /// Played at least once, but not within the last `days` days; the longest
    /// forgotten first.
    async fn forgotten(&self, days: i64, limit: i64) -> CoreResult<Vec<Track>> {
        let cutoff = now_ms().saturating_sub(days.saturating_mul(MS_PER_DAY));
        let rows = dyn_query(format!(
            "{} WHERE pc.last_played_at IS NOT NULL AND pc.last_played_at < ? \
             ORDER BY pc.last_played_at ASC, t.id ASC LIMIT ?",
            track_select()
        ))
        .bind(cutoff)
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;
        Ok(tracks_from_rows(&rows)?)
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteStatisticsRepository;
    use crate::domain::{ScannedTrack, StatsRange, MS_PER_DAY};
    use crate::infrastructure::repositories::{
        HistoryRepository, StatisticsRepository, TrackRepository,
    };
    use crate::infrastructure::sqlite::history_repo::SqliteHistoryRepository;
    use crate::infrastructure::sqlite::sql::{
        now_ms,
        test_support::{pool, scanned},
    };
    use crate::infrastructure::sqlite::track_repo::SqliteTrackRepository;

    struct Fixture {
        stats: SqliteStatisticsRepository,
        history: SqliteHistoryRepository,
        ids: Vec<i64>,
    }

    fn tagged(uri: &str, title: &str, artist: &str, album: &str, genre: &str) -> ScannedTrack {
        ScannedTrack {
            artist: Some(artist.to_owned()),
            album: Some(album.to_owned()),
            album_artist: Some(artist.to_owned()),
            genre: Some(genre.to_owned()),
            ..scanned(uri, title)
        }
    }

    /// Three 180 s tracks: Alpha, Bravo (both rock, same artist) and Charlie
    /// (jazz).
    async fn fixture() -> Fixture {
        let db = pool().await;
        let tracks = SqliteTrackRepository::new(db.clone());
        tracks
            .upsert_many(&[
                tagged("content://a", "Alpha", "Artist One", "First", "Rock"),
                tagged("content://b", "Bravo", "Artist One", "First", "rock"),
                tagged("content://c", "Charlie", "Artist Two", "Second", "Jazz"),
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

        Fixture {
            stats: SqliteStatisticsRepository::new(db.clone()),
            history: SqliteHistoryRepository::new(db),
            ids,
        }
    }

    #[tokio::test]
    async fn only_real_listens_are_counted() {
        let fixture = fixture().await;
        let now = now_ms();

        // Counts: full play, exactly the 30 s floor, and half of a 180 s track.
        fixture
            .history
            .record(fixture.ids[0], now, 180_000)
            .await
            .expect("full");
        fixture
            .history
            .record(fixture.ids[0], now, 30_000)
            .await
            .expect("floor");
        fixture
            .history
            .record(fixture.ids[1], now, 90_000)
            .await
            .expect("half");
        // Does not count: a 5 s skip, well under both thresholds.
        fixture
            .history
            .record(fixture.ids[2], now, 5_000)
            .await
            .expect("skip");

        let top = fixture
            .stats
            .top_tracks(StatsRange::AllTime, 10)
            .await
            .expect("top tracks");
        assert_eq!(top.len(), 2, "the 5 s skip does not make the list");
        assert_eq!(top[0].track.id, fixture.ids[0]);
        assert_eq!(top[0].plays, 2);
        assert_eq!(top[0].listening_time_ms, 210_000);
    }

    #[tokio::test]
    async fn the_window_excludes_older_history() {
        let fixture = fixture().await;
        let now = now_ms();

        fixture
            .history
            .record(fixture.ids[0], now - 100 * MS_PER_DAY, 180_000)
            .await
            .expect("old play");
        fixture
            .history
            .record(fixture.ids[1], now - MS_PER_DAY / 2, 180_000)
            .await
            .expect("recent play");

        let week = fixture
            .stats
            .top_tracks(StatsRange::Week, 10)
            .await
            .expect("top tracks");
        assert_eq!(week.len(), 1);
        assert_eq!(week[0].track.id, fixture.ids[1]);

        let all = fixture
            .stats
            .top_tracks(StatsRange::AllTime, 10)
            .await
            .expect("top tracks");
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn rankings_are_ordered_by_plays() {
        let fixture = fixture().await;
        let now = now_ms();

        for _ in 0..3 {
            fixture
                .history
                .record(fixture.ids[0], now, 180_000)
                .await
                .expect("alpha play");
        }
        fixture
            .history
            .record(fixture.ids[2], now, 180_000)
            .await
            .expect("charlie play");

        let tracks = fixture
            .stats
            .top_tracks(StatsRange::AllTime, 10)
            .await
            .expect("top tracks");
        assert_eq!(tracks.len(), 2);
        assert_eq!(tracks[0].track.title, "Alpha");
        assert_eq!(tracks[0].plays, 3);
        assert_eq!(tracks[0].listening_time_ms, 540_000);
        assert_eq!(tracks[0].track.play_count, 3, "the joined counter follows");

        let capped = fixture
            .stats
            .top_tracks(StatsRange::AllTime, 1)
            .await
            .expect("capped");
        assert_eq!(capped.len(), 1);
    }

    #[tokio::test]
    async fn never_played_and_forgotten_split_the_library() {
        let fixture = fixture().await;
        let now = now_ms();

        fixture
            .history
            .record(fixture.ids[0], now - 90 * MS_PER_DAY, 180_000)
            .await
            .expect("old play");
        fixture
            .history
            .record(fixture.ids[1], now, 180_000)
            .await
            .expect("fresh play");

        let never = fixture.stats.never_played(10).await.expect("never played");
        assert_eq!(
            never.iter().map(|track| track.id).collect::<Vec<_>>(),
            vec![fixture.ids[2]]
        );

        let forgotten = fixture.stats.forgotten(60, 10).await.expect("forgotten");
        assert_eq!(
            forgotten.iter().map(|track| track.id).collect::<Vec<_>>(),
            vec![fixture.ids[0]]
        );
        assert!(fixture
            .stats
            .forgotten(365, 10)
            .await
            .expect("forgotten")
            .is_empty());
    }

    #[tokio::test]
    async fn an_empty_history_is_all_zeroes() {
        let fixture = fixture().await;

        assert!(fixture
            .stats
            .top_tracks(StatsRange::AllTime, 10)
            .await
            .expect("top tracks")
            .is_empty());
        assert_eq!(
            fixture.stats.never_played(10).await.expect("never").len(),
            3
        );
    }
}
