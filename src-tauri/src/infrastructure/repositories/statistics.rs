use crate::domain::{RankedTrack, StatsRange, Track};
use crate::error::CoreResult;

#[async_trait::async_trait]
pub trait StatisticsRepository: Send + Sync {
    async fn top_tracks(&self, range: StatsRange, limit: i64) -> CoreResult<Vec<RankedTrack>>;
    async fn never_played(&self, limit: i64) -> CoreResult<Vec<Track>>;
    /// Tracks played at least once but not within the last `days` days.
    async fn forgotten(&self, days: i64, limit: i64) -> CoreResult<Vec<Track>>;
}
