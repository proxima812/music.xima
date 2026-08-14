//! Full-text search (CONTRACTS §5, `search_*`).
//!
//! The search box fires on every keystroke, so a blank query is answered here
//! and never reaches FTS5.

use std::sync::Arc;

use crate::application::validated_limit;
use crate::domain::{SearchResults, Track};
use crate::error::CoreResult;
use crate::infrastructure::repositories::SearchRepository;

pub struct SearchService {
    search: Arc<dyn SearchRepository>,
}

impl SearchService {
    pub fn new(search: Arc<dyn SearchRepository>) -> Self {
        Self { search }
    }

    /// Tracks, albums, artists and playlists matching `q`.
    pub async fn all(&self, q: &str, limit: i64) -> CoreResult<SearchResults> {
        let Some(query) = normalized(q) else {
            return Ok(SearchResults::default());
        };
        let limit = validated_limit(limit)?;
        self.search.all(&query, limit).await
    }

    pub async fn tracks(&self, q: &str, limit: i64) -> CoreResult<Vec<Track>> {
        let Some(query) = normalized(q) else {
            return Ok(Vec::new());
        };
        let limit = validated_limit(limit)?;
        self.search.tracks(&query, limit).await
    }
}

/// `None` for a query that has nothing to match on.
fn normalized(q: &str) -> Option<String> {
    let trimmed = q.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::SearchService;
    use crate::application::testing::track;
    use crate::domain::{SearchResults, Track, MAX_PAGE_LIMIT};
    use crate::error::CoreResult;
    use crate::infrastructure::repositories::SearchRepository;

    #[derive(Default)]
    struct FakeSearch {
        queries: Mutex<Vec<(String, i64)>>,
    }

    impl FakeSearch {
        fn queries(&self) -> Vec<(String, i64)> {
            self.queries.lock().expect("lock").clone()
        }
    }

    #[async_trait::async_trait]
    impl SearchRepository for FakeSearch {
        async fn all(&self, q: &str, limit: i64) -> CoreResult<SearchResults> {
            self.queries
                .lock()
                .expect("lock")
                .push((q.to_owned(), limit));
            Ok(SearchResults {
                tracks: vec![track(1)],
                ..SearchResults::default()
            })
        }

        async fn tracks(&self, q: &str, limit: i64) -> CoreResult<Vec<Track>> {
            self.queries
                .lock()
                .expect("lock")
                .push((q.to_owned(), limit));
            Ok(vec![track(1)])
        }
    }

    fn service() -> (SearchService, Arc<FakeSearch>) {
        let search = Arc::new(FakeSearch::default());
        (SearchService::new(search.clone()), search)
    }

    #[tokio::test]
    async fn a_blank_query_never_reaches_the_index() {
        let (service, search) = service();

        assert!(service.all("", 20).await.expect("empty").is_empty());
        assert!(service.all("   \n", 20).await.expect("blank").is_empty());
        assert!(service.tracks("", 20).await.expect("empty").is_empty());
        assert!(service.tracks("\t", 20).await.expect("blank").is_empty());

        assert!(search.queries().is_empty());
    }

    #[tokio::test]
    async fn a_blank_query_ignores_a_broken_limit_too() {
        let (service, search) = service();

        assert!(service.all("  ", 0).await.expect("blank").is_empty());
        assert!(service.tracks("  ", -1).await.expect("blank").is_empty());

        assert!(search.queries().is_empty());
    }

    #[tokio::test]
    async fn the_query_is_trimmed_and_the_limit_capped() {
        let (service, search) = service();

        let results = service.all("  radiohead ", 100_000).await.expect("results");
        assert_eq!(results.tracks.len(), 1);
        assert!(!results.is_empty());

        service.tracks(" bends ", 25).await.expect("track results");

        assert_eq!(
            search.queries(),
            vec![
                ("radiohead".to_owned(), MAX_PAGE_LIMIT),
                ("bends".to_owned(), 25),
            ]
        );
    }

    #[tokio::test]
    async fn a_non_positive_limit_is_rejected_for_a_real_query() {
        let (service, search) = service();

        assert_eq!(
            service
                .all("bends", 0)
                .await
                .expect_err("zero limit")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service
                .tracks("bends", -5)
                .await
                .expect_err("negative limit")
                .code(),
            "INVALID_INPUT"
        );
        assert!(search.queries().is_empty());
    }
}
