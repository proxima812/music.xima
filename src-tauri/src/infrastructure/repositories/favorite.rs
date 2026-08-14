use crate::domain::Track;
use crate::error::CoreResult;

#[async_trait::async_trait]
pub trait FavoriteRepository: Send + Sync {
    /// Flips the favorite flag and returns the new state.
    async fn toggle(&self, track_id: i64) -> CoreResult<bool>;
    async fn list(&self) -> CoreResult<Vec<Track>>;
}
