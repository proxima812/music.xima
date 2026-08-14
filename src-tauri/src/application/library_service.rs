//! Read side of the library (CONTRACTS §5, `library_*` plus `favorite_*`).
//!
//! Validates the untrusted numbers coming over IPC, then delegates. No SQL, no
//! filesystem: artwork resolution goes through [`ArtworkPort`].

use std::sync::Arc;

use crate::application::{
    validated_id, validated_limit, validated_offset, validated_track_ids, ArtworkPort,
};
use crate::domain::{
    Album, Artist, Folder, Genre, LibraryStats, Page, Track, TrackQuery, TrackSort, MAX_PAGE_LIMIT,
};
use crate::error::CoreResult;
use crate::infrastructure::repositories::{
    AlbumRepository, ArtistRepository, FavoriteRepository, TaxonomyRepository, TrackRepository,
};

pub struct LibraryService {
    tracks: Arc<dyn TrackRepository>,
    albums: Arc<dyn AlbumRepository>,
    artists: Arc<dyn ArtistRepository>,
    taxonomy: Arc<dyn TaxonomyRepository>,
    favorites: Arc<dyn FavoriteRepository>,
    artwork: Arc<dyn ArtworkPort>,
}

impl LibraryService {
    pub fn new(
        tracks: Arc<dyn TrackRepository>,
        albums: Arc<dyn AlbumRepository>,
        artists: Arc<dyn ArtistRepository>,
        taxonomy: Arc<dyn TaxonomyRepository>,
        favorites: Arc<dyn FavoriteRepository>,
        artwork: Arc<dyn ArtworkPort>,
    ) -> Self {
        Self {
            tracks,
            albums,
            artists,
            taxonomy,
            favorites,
            artwork,
        }
    }

    pub async fn stats(&self) -> CoreResult<LibraryStats> {
        self.tracks.stats().await
    }

    pub async fn tracks(&self, query: &TrackQuery) -> CoreResult<Page<Track>> {
        validated_offset(query.offset)?;
        validated_limit(query.limit)?;
        self.tracks.query(&query.normalized()).await
    }

    pub async fn track(&self, id: i64) -> CoreResult<Track> {
        let id = validated_id("track", id)?;
        self.tracks.get(id).await
    }

    /// Missing ids are skipped: the caller usually holds a stale queue or a
    /// stale selection, and a partial answer is more useful than an error.
    pub async fn tracks_by_ids(&self, ids: &[i64]) -> CoreResult<Vec<Track>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        validated_track_ids(ids)?;
        self.tracks.get_many(ids).await
    }

    pub async fn recently_added(&self, limit: i64) -> CoreResult<Vec<Track>> {
        let limit = validated_limit(limit)?;
        self.tracks.recently_added(limit).await
    }

    pub async fn albums(&self, offset: i64, limit: i64) -> CoreResult<Page<Album>> {
        let offset = validated_offset(offset)?;
        let limit = validated_limit(limit)?;
        self.albums.query(offset, limit).await
    }

    pub async fn album(&self, id: i64) -> CoreResult<Album> {
        let id = validated_id("album", id)?;
        self.albums.get(id).await
    }

    /// Every track of an album, in album order (disc, then track number).
    pub async fn album_tracks(&self, album_id: i64) -> CoreResult<Vec<Track>> {
        let album_id = validated_id("album", album_id)?;
        self.collect(TrackQuery {
            sort: TrackSort::AlbumAsc,
            album_id: Some(album_id),
            limit: MAX_PAGE_LIMIT,
            ..TrackQuery::default()
        })
        .await
    }

    pub async fn artists(&self, offset: i64, limit: i64) -> CoreResult<Page<Artist>> {
        let offset = validated_offset(offset)?;
        let limit = validated_limit(limit)?;
        self.artists.query(offset, limit).await
    }

    pub async fn artist(&self, id: i64) -> CoreResult<Artist> {
        let id = validated_id("artist", id)?;
        self.artists.get(id).await
    }

    pub async fn artist_albums(&self, artist_id: i64) -> CoreResult<Vec<Album>> {
        let artist_id = validated_id("artist", artist_id)?;
        self.albums.by_artist(artist_id).await
    }

    /// Every track of an artist, grouped by album.
    pub async fn artist_tracks(&self, artist_id: i64) -> CoreResult<Vec<Track>> {
        let artist_id = validated_id("artist", artist_id)?;
        self.collect(TrackQuery {
            sort: TrackSort::AlbumAsc,
            artist_id: Some(artist_id),
            limit: MAX_PAGE_LIMIT,
            ..TrackQuery::default()
        })
        .await
    }

    pub async fn genres(&self) -> CoreResult<Vec<Genre>> {
        self.taxonomy.genres().await
    }

    /// Direct children of `parent`; top-level folders when `parent` is `None`.
    pub async fn folders(&self, parent: Option<&str>) -> CoreResult<Vec<Folder>> {
        let parent = parent.map(str::trim).filter(|value| !value.is_empty());
        self.taxonomy.folders(parent).await
    }

    /// A blank key means "this track has no artwork", not "bad request".
    pub async fn artwork_uri(&self, cover_key: &str) -> CoreResult<Option<String>> {
        let key = cover_key.trim();
        if key.is_empty() {
            return Ok(None);
        }
        self.artwork.artwork_uri(key).await
    }

    /// Returns the new favorite state.
    pub async fn toggle_favorite(&self, track_id: i64) -> CoreResult<bool> {
        let track_id = validated_id("track", track_id)?;
        self.favorites.toggle(track_id).await
    }

    pub async fn favorites(&self) -> CoreResult<Vec<Track>> {
        self.favorites.list().await
    }

    /// Drains every page of `query`. Used where the contract returns a plain
    /// `Vec<Track>` (album / artist tracks) but the repository is paginated.
    async fn collect(&self, query: TrackQuery) -> CoreResult<Vec<Track>> {
        let mut query = query.normalized();
        let mut collected: Vec<Track> = Vec::new();
        loop {
            let page = self.tracks.query(&query).await?;
            let fetched = page.items.len();
            collected.extend(page.items);
            if fetched == 0 || collected.len() as i64 >= page.total {
                return Ok(collected);
            }
            query.offset = query.offset.saturating_add(fetched as i64);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::LibraryService;
    use crate::application::testing::{track, CacheArtwork};
    use crate::application::ArtworkPort;
    use crate::domain::{
        Album, Artist, Folder, Genre, LibraryStats, Page, ScannedTrack, Track, TrackQuery,
        TrackSort, MAX_PAGE_LIMIT,
    };
    use crate::error::{CoreError, CoreResult};
    use crate::infrastructure::repositories::{
        AlbumRepository, ArtistRepository, FavoriteRepository, TaxonomyRepository, TrackRepository,
    };

    #[derive(Default)]
    struct FakeTracks {
        queries: Mutex<Vec<TrackQuery>>,
        pages: Mutex<Vec<Page<Track>>>,
        get_many_calls: Mutex<Vec<Vec<i64>>>,
    }

    impl FakeTracks {
        fn with_pages(pages: Vec<Page<Track>>) -> Self {
            Self {
                pages: Mutex::new(pages),
                ..Self::default()
            }
        }

        fn recorded_queries(&self) -> Vec<TrackQuery> {
            self.queries.lock().expect("lock").clone()
        }
    }

    #[async_trait::async_trait]
    impl TrackRepository for FakeTracks {
        async fn get(&self, id: i64) -> CoreResult<Track> {
            Ok(track(id))
        }

        async fn get_many(&self, ids: &[i64]) -> CoreResult<Vec<Track>> {
            self.get_many_calls.lock().expect("lock").push(ids.to_vec());
            Ok(ids.iter().copied().map(track).collect())
        }

        async fn query(&self, q: &TrackQuery) -> CoreResult<Page<Track>> {
            self.queries.lock().expect("lock").push(q.clone());
            let mut pages = self.pages.lock().expect("lock");
            if pages.is_empty() {
                return Ok(Page::empty(q.offset, q.limit));
            }
            Ok(pages.remove(0))
        }

        async fn recently_added(&self, limit: i64) -> CoreResult<Vec<Track>> {
            Ok((1..=limit).map(track).collect())
        }

        async fn upsert_many(&self, _tracks: &[ScannedTrack]) -> CoreResult<u64> {
            Ok(0)
        }

        async fn delete_missing(&self, _keep_uris: &[String]) -> CoreResult<u64> {
            Ok(0)
        }

        async fn stats(&self) -> CoreResult<LibraryStats> {
            Ok(LibraryStats {
                tracks: 3,
                albums: 1,
                artists: 1,
                playlists: 0,
                genres: 1,
                total_duration_ms: 600_000,
                total_size: 24_000_000,
            })
        }
    }

    struct FakeAlbums;

    #[async_trait::async_trait]
    impl AlbumRepository for FakeAlbums {
        async fn get(&self, id: i64) -> CoreResult<Album> {
            Ok(album(id))
        }

        async fn query(&self, offset: i64, limit: i64) -> CoreResult<Page<Album>> {
            Ok(Page::new(vec![album(1)], 1, offset, limit))
        }

        async fn by_artist(&self, _artist_id: i64) -> CoreResult<Vec<Album>> {
            Ok(vec![album(1)])
        }
    }

    struct FakeArtists;

    #[async_trait::async_trait]
    impl ArtistRepository for FakeArtists {
        async fn get(&self, id: i64) -> CoreResult<Artist> {
            Ok(Artist {
                id,
                name: "Artist".to_owned(),
                album_count: 1,
                track_count: 3,
                cover_key: None,
            })
        }

        async fn query(&self, offset: i64, limit: i64) -> CoreResult<Page<Artist>> {
            Ok(Page::new(Vec::new(), 0, offset, limit))
        }
    }

    #[derive(Default)]
    struct FakeTaxonomy {
        parents: Mutex<Vec<Option<String>>>,
    }

    #[async_trait::async_trait]
    impl TaxonomyRepository for FakeTaxonomy {
        async fn genres(&self) -> CoreResult<Vec<Genre>> {
            Ok(vec![Genre {
                name: "Rock".to_owned(),
                track_count: 3,
            }])
        }

        async fn folders(&self, parent: Option<&str>) -> CoreResult<Vec<Folder>> {
            self.parents
                .lock()
                .expect("lock")
                .push(parent.map(str::to_owned));
            Ok(vec![Folder::from_path("Music/Rock", 3)])
        }
    }

    #[derive(Default)]
    struct FakeFavorites {
        toggled: Mutex<Vec<i64>>,
    }

    #[async_trait::async_trait]
    impl FavoriteRepository for FakeFavorites {
        async fn toggle(&self, track_id: i64) -> CoreResult<bool> {
            self.toggled.lock().expect("lock").push(track_id);
            Ok(true)
        }

        async fn list(&self) -> CoreResult<Vec<Track>> {
            Ok(vec![track(1)])
        }
    }

    struct FailingArtwork;

    #[async_trait::async_trait]
    impl ArtworkPort for FailingArtwork {
        async fn artwork_uri(&self, cover_key: &str) -> CoreResult<Option<String>> {
            Err(CoreError::internal(format!("must not resolve {cover_key}")))
        }
    }

    fn album(id: i64) -> Album {
        Album {
            id,
            title: "Album".to_owned(),
            artist_id: Some(1),
            artist_name: Some("Artist".to_owned()),
            year: Some(2020),
            cover_key: None,
            track_count: 3,
            duration_ms: 600_000,
        }
    }

    fn service(tracks: Arc<FakeTracks>) -> LibraryService {
        LibraryService::new(
            tracks,
            Arc::new(FakeAlbums),
            Arc::new(FakeArtists),
            Arc::new(FakeTaxonomy::default()),
            Arc::new(FakeFavorites::default()),
            Arc::new(CacheArtwork),
        )
    }

    #[tokio::test]
    async fn rejects_negative_offset_and_non_positive_limit() {
        let tracks = Arc::new(FakeTracks::default());
        let service = service(tracks.clone());

        let query = TrackQuery {
            offset: -1,
            ..TrackQuery::default()
        };
        assert_eq!(
            service
                .tracks(&query)
                .await
                .expect_err("negative offset")
                .code(),
            "INVALID_INPUT"
        );

        let query = TrackQuery {
            limit: 0,
            ..TrackQuery::default()
        };
        assert_eq!(
            service.tracks(&query).await.expect_err("zero limit").code(),
            "INVALID_INPUT"
        );

        assert_eq!(
            service.albums(-1, 10).await.expect_err("bad offset").code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service.artists(0, -3).await.expect_err("bad limit").code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            service.track(0).await.expect_err("zero id").code(),
            "INVALID_INPUT"
        );
        assert!(tracks.recorded_queries().is_empty());
    }

    #[tokio::test]
    async fn caps_oversized_limit_before_hitting_the_repository() {
        let tracks = Arc::new(FakeTracks::default());
        let service = service(tracks.clone());
        let query = TrackQuery {
            limit: 100_000,
            ..TrackQuery::default()
        };

        service.tracks(&query).await.expect("query runs");

        let recorded = tracks.recorded_queries();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].limit, MAX_PAGE_LIMIT);
    }

    #[tokio::test]
    async fn tracks_by_ids_short_circuits_on_empty_input() {
        let tracks = Arc::new(FakeTracks::default());
        let service = service(tracks.clone());

        assert!(service
            .tracks_by_ids(&[])
            .await
            .expect("empty is fine")
            .is_empty());
        assert!(tracks.get_many_calls.lock().expect("lock").is_empty());

        assert_eq!(
            service
                .tracks_by_ids(&[1, -2])
                .await
                .expect_err("negative id")
                .code(),
            "INVALID_INPUT"
        );
    }

    #[tokio::test]
    async fn album_tracks_drain_every_page() {
        let first = Page::new(
            (1..=MAX_PAGE_LIMIT).map(track).collect::<Vec<Track>>(),
            MAX_PAGE_LIMIT + 2,
            0,
            MAX_PAGE_LIMIT,
        );
        let second = Page::new(
            vec![track(MAX_PAGE_LIMIT + 1), track(MAX_PAGE_LIMIT + 2)],
            MAX_PAGE_LIMIT + 2,
            MAX_PAGE_LIMIT,
            MAX_PAGE_LIMIT,
        );
        let tracks = Arc::new(FakeTracks::with_pages(vec![first, second]));
        let service = service(tracks.clone());

        let collected = service.album_tracks(7).await.expect("album tracks");

        assert_eq!(collected.len() as i64, MAX_PAGE_LIMIT + 2);
        let recorded = tracks.recorded_queries();
        assert_eq!(recorded.len(), 2);
        assert_eq!(recorded[0].album_id, Some(7));
        assert_eq!(recorded[0].sort, TrackSort::AlbumAsc);
        assert_eq!(recorded[0].offset, 0);
        assert_eq!(recorded[1].offset, MAX_PAGE_LIMIT);
    }

    #[tokio::test]
    async fn artist_tracks_stop_when_a_page_comes_back_empty() {
        let tracks = Arc::new(FakeTracks::with_pages(vec![Page::new(
            vec![track(1)],
            99,
            0,
            MAX_PAGE_LIMIT,
        )]));
        let service = service(tracks.clone());

        let collected = service.artist_tracks(3).await.expect("artist tracks");

        assert_eq!(collected.len(), 1);
        assert_eq!(tracks.recorded_queries().len(), 2);
    }

    #[tokio::test]
    async fn artwork_key_is_trimmed_and_blank_keys_never_reach_the_port() {
        let failing = LibraryService::new(
            Arc::new(FakeTracks::default()),
            Arc::new(FakeAlbums),
            Arc::new(FakeArtists),
            Arc::new(FakeTaxonomy::default()),
            Arc::new(FakeFavorites::default()),
            Arc::new(FailingArtwork),
        );

        assert_eq!(failing.artwork_uri("   ").await.expect("blank key"), None);
        assert_eq!(failing.artwork_uri("").await.expect("empty key"), None);
        assert!(failing.artwork_uri("cover-1").await.is_err());

        let resolving = service(Arc::new(FakeTracks::default()));
        assert_eq!(
            resolving.artwork_uri(" cover-1 ").await.expect("resolved"),
            Some("file:///cache/cover-1.jpg".to_owned())
        );
    }

    #[tokio::test]
    async fn blank_folder_parent_becomes_top_level() {
        let taxonomy = Arc::new(FakeTaxonomy::default());
        let service = LibraryService::new(
            Arc::new(FakeTracks::default()),
            Arc::new(FakeAlbums),
            Arc::new(FakeArtists),
            taxonomy.clone(),
            Arc::new(FakeFavorites::default()),
            Arc::new(CacheArtwork),
        );

        service.folders(Some("  ")).await.expect("folders");
        service.folders(Some(" Music ")).await.expect("folders");
        service.folders(None).await.expect("folders");

        let recorded = taxonomy.parents.lock().expect("lock").clone();
        assert_eq!(
            recorded,
            vec![None, Some("Music".to_owned()), None],
            "blank parents are normalized away"
        );
    }

    #[tokio::test]
    async fn favorites_are_validated_and_delegated() {
        let favorites = Arc::new(FakeFavorites::default());
        let service = LibraryService::new(
            Arc::new(FakeTracks::default()),
            Arc::new(FakeAlbums),
            Arc::new(FakeArtists),
            Arc::new(FakeTaxonomy::default()),
            favorites.clone(),
            Arc::new(CacheArtwork),
        );

        assert!(service.toggle_favorite(4).await.expect("toggled"));
        assert_eq!(
            service
                .toggle_favorite(0)
                .await
                .expect_err("zero id")
                .code(),
            "INVALID_INPUT"
        );
        assert_eq!(service.favorites().await.expect("list").len(), 1);
        assert_eq!(favorites.toggled.lock().expect("lock").as_slice(), &[4]);
    }

    #[tokio::test]
    async fn stats_and_recently_added_pass_through() {
        let service = service(Arc::new(FakeTracks::default()));
        assert_eq!(service.stats().await.expect("stats").tracks, 3);
        assert_eq!(
            service.recently_added(5).await.expect("recent").len(),
            5,
            "limit reaches the repository untouched"
        );
        assert_eq!(
            service
                .recently_added(0)
                .await
                .expect_err("zero limit")
                .code(),
            "INVALID_INPUT"
        );
    }
}
