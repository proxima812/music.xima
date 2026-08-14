use crate::domain::Track;
use crate::error::CoreResult;

#[async_trait::async_trait]
pub trait HistoryRepository: Send + Sync {
    /// Appends a history row and bumps `play_counts` for the track.
    async fn record(
        &self,
        track_id: i64,
        played_at: i64,
        duration_played_ms: i64,
    ) -> CoreResult<()>;
    async fn recent(&self, limit: i64) -> CoreResult<Vec<Track>>;
}
