use crate::domain::{Playlist, Track};
use crate::error::CoreResult;

#[async_trait::async_trait]
pub trait PlaylistRepository: Send + Sync {
    async fn list(&self) -> CoreResult<Vec<Playlist>>;
    async fn get(&self, id: i64) -> CoreResult<Playlist>;
    async fn create(&self, name: &str) -> CoreResult<Playlist>;
    async fn rename(&self, id: i64, name: &str) -> CoreResult<()>;
    async fn delete(&self, id: i64) -> CoreResult<()>;
    /// Tracks in playlist order.
    async fn tracks(&self, id: i64) -> CoreResult<Vec<Track>>;
    async fn add_tracks(&self, id: i64, track_ids: &[i64]) -> CoreResult<()>;
    async fn remove_at(&self, id: i64, position: i64) -> CoreResult<()>;
    async fn reorder(&self, id: i64, from: i64, to: i64) -> CoreResult<()>;
}
