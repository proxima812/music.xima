use crate::domain::{
    HiddenTrack, LibraryStats, Page, PendingDeletion, ScannedTrack, Track, TrackQuery,
};
use crate::error::{CoreError, CoreResult};

#[async_trait::async_trait]
pub trait TrackRepository: Send + Sync {
    async fn get(&self, id: i64) -> CoreResult<Track>;
    async fn get_many(&self, ids: &[i64]) -> CoreResult<Vec<Track>>;
    async fn query(&self, q: &TrackQuery) -> CoreResult<Page<Track>>;
    async fn recently_added(&self, limit: i64) -> CoreResult<Vec<Track>>;
    /// Returns `true` only when this call inserted the tombstone.
    async fn hide(&self, _track_id: i64, _hidden_at: i64) -> CoreResult<bool> {
        Err(CoreError::internal("track removal is not implemented"))
    }
    async fn restore(&self, _track_id: i64) -> CoreResult<()> {
        Err(CoreError::internal("track removal is not implemented"))
    }
    async fn hidden(&self) -> CoreResult<Vec<HiddenTrack>> {
        Err(CoreError::internal("track removal is not implemented"))
    }
    async fn begin_deletion(
        &self,
        _track_id: i64,
        _requested_at: i64,
    ) -> CoreResult<PendingDeletion> {
        Err(CoreError::internal("track removal is not implemented"))
    }
    async fn cancel_deletion(&self, _track_id: i64) -> CoreResult<()> {
        Err(CoreError::internal("track removal is not implemented"))
    }
    async fn mark_file_deleted(&self, _track_id: i64) -> CoreResult<()> {
        Err(CoreError::internal("track removal is not implemented"))
    }
    async fn finalize_deletion(&self, _track_id: i64) -> CoreResult<()> {
        Err(CoreError::internal("track removal is not implemented"))
    }
    async fn pending_deletions(&self) -> CoreResult<Vec<PendingDeletion>> {
        Err(CoreError::internal("track removal is not implemented"))
    }
    /// Inserts new tracks and refreshes existing ones, keyed by `uri`.
    /// Returns how many rows were written.
    async fn upsert_many(&self, tracks: &[ScannedTrack]) -> CoreResult<u64>;
    /// Drops every track whose `uri` is not in `keep_uris`. Returns how many
    /// rows were removed.
    async fn delete_missing(&self, keep_uris: &[String]) -> CoreResult<u64>;
    async fn stats(&self) -> CoreResult<LibraryStats>;
}
