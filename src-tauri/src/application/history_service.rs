//! Listening history (CONTRACTS §5, `history_*`).
//!
//! Rust owns this: the native player reports `player:completed`, the command
//! layer forwards it here. The `history_record` command exists for manual
//! scenarios and tests.

use std::sync::Arc;

use crate::application::{validated_id, validated_limit, Clock, SystemClock};
use crate::domain::Track;
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::HistoryRepository;

pub struct HistoryService {
    history: Arc<dyn HistoryRepository>,
    clock: Arc<dyn Clock>,
}

impl HistoryService {
    pub fn new(history: Arc<dyn HistoryRepository>) -> Self {
        Self {
            history,
            clock: Arc::new(SystemClock),
        }
    }

    pub fn with_clock(history: Arc<dyn HistoryRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { history, clock }
    }

    /// A track finished or was skipped away from: stamp it with the current
    /// time and write it down.
    pub async fn on_track_completed(
        &self,
        track_id: i64,
        duration_played_ms: i64,
    ) -> CoreResult<()> {
        self.record(track_id, self.clock.now_ms(), duration_played_ms)
            .await
    }

    pub async fn record(
        &self,
        track_id: i64,
        played_at: i64,
        duration_played_ms: i64,
    ) -> CoreResult<()> {
        let track_id = validated_id("track", track_id)?;
        if played_at <= 0 {
            return Err(CoreError::invalid_input(
                "playedAt must be a Unix timestamp",
            ));
        }
        if duration_played_ms < 0 {
            return Err(CoreError::invalid_input(
                "durationPlayedMs must not be negative",
            ));
        }
        self.history
            .record(track_id, played_at, duration_played_ms)
            .await
    }

    pub async fn recent(&self, limit: i64) -> CoreResult<Vec<Track>> {
        let limit = validated_limit(limit)?;
        self.history.recent(limit).await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::HistoryService;
    use crate::application::testing::{track, FixedClock};
    use crate::domain::{Track, MAX_PAGE_LIMIT};
    use crate::error::CoreResult;
    use crate::infrastructure::repositories::HistoryRepository;

    const NOW: i64 = 1_700_000_000_000;

    #[derive(Default)]
    struct FakeHistory {
        records: Mutex<Vec<(i64, i64, i64)>>,
        limits: Mutex<Vec<i64>>,
    }

    #[async_trait::async_trait]
    impl HistoryRepository for FakeHistory {
        async fn record(
            &self,
            track_id: i64,
            played_at: i64,
            duration_played_ms: i64,
        ) -> CoreResult<()> {
            self.records
                .lock()
                .expect("lock")
                .push((track_id, played_at, duration_played_ms));
            Ok(())
        }

        async fn recent(&self, limit: i64) -> CoreResult<Vec<Track>> {
            self.limits.lock().expect("lock").push(limit);
            Ok(vec![track(1)])
        }
    }

    fn service() -> (HistoryService, Arc<FakeHistory>) {
        let history = Arc::new(FakeHistory::default());
        (
            HistoryService::with_clock(history.clone(), Arc::new(FixedClock(NOW))),
            history,
        )
    }

    #[tokio::test]
    async fn completion_is_stamped_with_the_current_time() {
        let (service, history) = service();

        service
            .on_track_completed(42, 180_000)
            .await
            .expect("recorded");

        assert_eq!(
            history.records.lock().expect("lock").as_slice(),
            &[(42, NOW, 180_000)]
        );
    }

    #[tokio::test]
    async fn zero_length_plays_are_still_recorded() {
        let (service, history) = service();

        service.on_track_completed(42, 0).await.expect("recorded");

        assert_eq!(history.records.lock().expect("lock").len(), 1);
    }

    #[tokio::test]
    async fn rejects_bad_ids_timestamps_and_durations() {
        let (service, history) = service();

        assert_eq!(
            service
                .on_track_completed(0, 1_000)
                .await
                .expect_err("zero track id")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .on_track_completed(1, -1)
                .await
                .expect_err("negative duration")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .record(1, 0, 1_000)
                .await
                .expect_err("missing timestamp")
                .code(),
            "INVALID_INPUT"
        );

        assert!(history.records.lock().expect("lock").is_empty());
    }

    #[tokio::test]
    async fn recent_validates_and_caps_the_limit() {
        let (service, history) = service();

        service.recent(10).await.expect("recent");
        service.recent(100_000).await.expect("recent");
        assert_eq!(
            service.recent(0).await.expect_err("zero limit").code(),
            "INVALID_INPUT"
        );

        assert_eq!(
            history.limits.lock().expect("lock").as_slice(),
            &[10, MAX_PAGE_LIMIT]
        );
    }
}
