use crate::domain::{Artist, Page};
use crate::error::CoreResult;

#[async_trait::async_trait]
pub trait ArtistRepository: Send + Sync {
    async fn get(&self, id: i64) -> CoreResult<Artist>;
    async fn query(&self, offset: i64, limit: i64) -> CoreResult<Page<Artist>>;
}
