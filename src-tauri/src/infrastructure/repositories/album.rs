use crate::domain::{Album, Page};
use crate::error::CoreResult;

#[async_trait::async_trait]
pub trait AlbumRepository: Send + Sync {
    async fn get(&self, id: i64) -> CoreResult<Album>;
    async fn query(&self, offset: i64, limit: i64) -> CoreResult<Page<Album>>;
    async fn by_artist(&self, artist_id: i64) -> CoreResult<Vec<Album>>;
}
