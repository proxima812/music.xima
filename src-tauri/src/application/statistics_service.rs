//! Track picks derived from history (CONTRACTS §5, `stats_*`).
//!
//! The range windows themselves are SQL (`StatsRange::since_ms` mirrors them);
//! this layer only guards the numbers coming from the frontend.

use std::sync::Arc;

use crate::application::validated_limit;
use crate::domain::{RankedTrack, StatsRange, Track};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::StatisticsRepository;

/// Guard for `stats_forgotten`: "not played in the last N days" is meaningless
/// beyond a few decades and usually means the frontend sent milliseconds.
const MAX_FORGOTTEN_DAYS: i64 = 36_500;

pub struct StatisticsService {
    stats: Arc<dyn StatisticsRepository>,
}

impl StatisticsService {
    pub fn new(stats: Arc<dyn StatisticsRepository>) -> Self {
        Self { stats }
    }

    pub async fn top_tracks(&self, range: StatsRange, limit: i64) -> CoreResult<Vec<RankedTrack>> {
        let limit = validated_limit(limit)?;
        self.stats.top_tracks(range, limit).await
    }

    pub async fn never_played(&self, limit: i64) -> CoreResult<Vec<Track>> {
        let limit = validated_limit(limit)?;
        self.stats.never_played(limit).await
    }

    pub async fn forgotten(&self, days: i64, limit: i64) -> CoreResult<Vec<Track>> {
        if days <= 0 || days > MAX_FORGOTTEN_DAYS {
            return Err(CoreError::invalid_input(format!(
                "days must be between 1 and {MAX_FORGOTTEN_DAYS}"
            )));
        }
        let limit = validated_limit(limit)?;
        self.stats.forgotten(days, limit).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::{StatisticsService, MAX_FORGOTTEN_DAYS};
    use crate::application::testing::track;
    use crate::domain::{RankedTrack, StatsRange, Track, MAX_PAGE_LIMIT};
    use crate::error::CoreResult;
    use crate::infrastructure::repositories::StatisticsRepository;

    #[derive(Default)]
    struct FakeStats {
        limits: Mutex<Vec<(&'static str, StatsRange, i64)>>,
        forgotten: Mutex<Vec<(i64, i64)>>,
    }

    #[async_trait::async_trait]
    impl StatisticsRepository for FakeStats {
        async fn top_tracks(&self, range: StatsRange, limit: i64) -> CoreResult<Vec<RankedTrack>> {
            self.limits
                .lock()
                .expect("lock")
                .push(("tracks", range, limit));
            Ok(vec![RankedTrack {
                track: track(1),
                plays: 3,
                listening_time_ms: 600_000,
            }])
        }

        async fn never_played(&self, limit: i64) -> CoreResult<Vec<Track>> {
            self.limits
                .lock()
                .expect("lock")
                .push(("never", StatsRange::AllTime, limit));
            Ok(Vec::new())
        }

        async fn forgotten(&self, days: i64, limit: i64) -> CoreResult<Vec<Track>> {
            self.forgotten.lock().expect("lock").push((days, limit));
            Ok(vec![track(2)])
        }
    }

    fn service() -> (StatisticsService, Arc<FakeStats>) {
        let stats = Arc::new(FakeStats::default());
        (StatisticsService::new(stats.clone()), stats)
    }

    #[tokio::test]
    async fn every_list_caps_its_limit() {
        let (service, stats) = service();

        service
            .top_tracks(StatsRange::Today, 100_000)
            .await
            .expect("tracks");
        service.top_tracks(StatsRange::Month, 10).await.expect("ten");
        service.never_played(1_000).await.expect("never played");

        let recorded = stats.limits.lock().expect("lock").clone();
        assert_eq!(
            recorded,
            vec![
                ("tracks", StatsRange::Today, MAX_PAGE_LIMIT),
                ("tracks", StatsRange::Month, 10),
                ("never", StatsRange::AllTime, MAX_PAGE_LIMIT),
            ]
        );
    }

    #[tokio::test]
    async fn non_positive_limits_are_rejected() {
        let (service, stats) = service();

        assert_eq!(
            service
                .top_tracks(StatsRange::Today, 0)
                .await
                .expect_err("zero limit")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .top_tracks(StatsRange::Today, -1)
                .await
                .expect_err("negative limit")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .never_played(0)
                .await
                .expect_err("zero limit")
                .code(),
            "INVALID_INPUT"
        );
        assert!(stats.limits.lock().expect("lock").is_empty());
    }

    #[tokio::test]
    async fn forgotten_requires_a_sane_day_window() {
        let (service, stats) = service();

        assert_eq!(
            service
                .forgotten(0, 10)
                .await
                .expect_err("zero days")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .forgotten(MAX_FORGOTTEN_DAYS + 1, 10)
                .await
                .expect_err("absurd window")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .forgotten(60, 0)
                .await
                .expect_err("zero limit")
                .code(),
            "INVALID_INPUT"
        );

        let tracks = service.forgotten(60, 10).await.expect("forgotten");

        assert_eq!(tracks.len(), 1);
        assert_eq!(
            stats.forgotten.lock().expect("lock").as_slice(),
            &[(60, 10)]
        );
    }
}
