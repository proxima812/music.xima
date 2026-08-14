use crate::domain::{SmartPlaylist, SmartPlaylistDraft, SmartRule, SmartSort, Track};
use crate::error::CoreResult;

#[async_trait::async_trait]
pub trait SmartPlaylistRepository: Send + Sync {
    async fn list(&self) -> CoreResult<Vec<SmartPlaylist>>;
    async fn get(&self, id: i64) -> CoreResult<SmartPlaylist>;
    async fn create(&self, draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylist>;
    async fn update(&self, id: i64, draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylist>;
    async fn delete(&self, id: i64) -> CoreResult<()>;
    /// Compiles the rules into SQL and returns the matching tracks.
    async fn resolve(
        &self,
        rules: &[SmartRule],
        match_all: bool,
        sort: SmartSort,
        limit: Option<i64>,
    ) -> CoreResult<Vec<Track>>;
}
