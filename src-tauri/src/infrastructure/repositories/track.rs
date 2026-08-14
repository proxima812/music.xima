use crate::domain::{LibraryStats, Page, ScannedTrack, Track, TrackQuery};
use crate::error::CoreResult;

#[async_trait::async_trait]
pub trait TrackRepository: Send + Sync {
    async fn get(&self, id: i64) -> CoreResult<Track>;
    async fn get_many(&self, ids: &[i64]) -> CoreResult<Vec<Track>>;
    async fn query(&self, q: &TrackQuery) -> CoreResult<Page<Track>>;
    async fn recently_added(&self, limit: i64) -> CoreResult<Vec<Track>>;
    /// Inserts new tracks and refreshes existing ones, keyed by `uri`.
    /// Returns how many rows were written.
    async fn upsert_many(&self, tracks: &[ScannedTrack]) -> CoreResult<u64>;
    /// Drops every track whose `uri` is not in `keep_uris`. Returns how many
    /// rows were removed.
    async fn delete_missing(&self, keep_uris: &[String]) -> CoreResult<u64>;
    async fn stats(&self) -> CoreResult<LibraryStats>;
}
