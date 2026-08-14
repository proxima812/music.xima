use crate::domain::{Folder, Genre};
use crate::error::CoreResult;

#[async_trait::async_trait]
pub trait TaxonomyRepository: Send + Sync {
    async fn genres(&self) -> CoreResult<Vec<Genre>>;
    /// Direct children of `parent`; top-level folders when `parent` is `None`.
    async fn folders(&self, parent: Option<&str>) -> CoreResult<Vec<Folder>>;
}
