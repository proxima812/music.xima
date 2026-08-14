use crate::domain::{SearchResults, Track};
use crate::error::CoreResult;

#[async_trait::async_trait]
pub trait SearchRepository: Send + Sync {
    /// `limit` applies to each bucket of the result, not to their sum.
    async fn all(&self, q: &str, limit: i64) -> CoreResult<SearchResults>;
    async fn tracks(&self, q: &str, limit: i64) -> CoreResult<Vec<Track>>;
}
