//! `SearchRepository` on SQLite (CONTRACTS §3).
//!
//! Tracks come from the `tracks_fts` index, which is where a user query can be
//! matched against title, artist, album and genre at once. Albums, artists and
//! playlists are small enough to scan with a `LIKE`.

use crate::domain::{clamp_limit, Album, Artist, Playlist, SearchResults, Track};
use crate::error::CoreResult;
use crate::infrastructure::repositories::SearchRepository;

use super::pool::Db;
use super::sql::{
    album_from_row, artist_from_row, dyn_query, fts_match, like_contains, playlist_from_row,
    tracks_from_rows, ALBUM_SELECT, ARTIST_SELECT, PLAYLIST_SELECT, TRACK_COLUMNS, TRACK_JOINS,
};

pub struct SqliteSearchRepository {
    pool: Db,
}

impl SqliteSearchRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }

    async fn matching_tracks(&self, q: &str, limit: i64) -> CoreResult<Vec<Track>> {
        let Some(expression) = fts_match(q) else {
            return Ok(Vec::new());
        };

        let rows = dyn_query(format!(
            "SELECT {TRACK_COLUMNS} FROM tracks_fts \
             JOIN tracks t ON t.id = tracks_fts.rowid {TRACK_JOINS} \
             WHERE tracks_fts MATCH ? \
             ORDER BY bm25(tracks_fts), t.sort_title ASC, t.id ASC LIMIT ?"
        ))
        .bind(expression)
        .bind(clamp_limit(limit))
        .fetch_all(&self.pool)
        .await?;
        Ok(tracks_from_rows(&rows)?)
    }

    async fn matching_albums(&self, pattern: &str, limit: i64) -> CoreResult<Vec<Album>> {
        let rows = dyn_query(format!(
            "{ALBUM_SELECT} WHERE al.title LIKE ? ESCAPE '\\' \
             GROUP BY al.id ORDER BY al.sort_title ASC, al.id ASC LIMIT ?"
        ))
        .bind(pattern.to_owned())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut albums = Vec::with_capacity(rows.len());
        for row in &rows {
            albums.push(album_from_row(row)?);
        }
        Ok(albums)
    }

    async fn matching_artists(&self, pattern: &str, limit: i64) -> CoreResult<Vec<Artist>> {
        let rows = dyn_query(format!(
            "{ARTIST_SELECT} WHERE ar.name LIKE ? ESCAPE '\\' \
             ORDER BY ar.sort_name ASC, ar.id ASC LIMIT ?"
        ))
        .bind(pattern.to_owned())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut artists = Vec::with_capacity(rows.len());
        for row in &rows {
            artists.push(artist_from_row(row)?);
        }
        Ok(artists)
    }

    async fn matching_playlists(&self, pattern: &str, limit: i64) -> CoreResult<Vec<Playlist>> {
        let rows = dyn_query(format!(
            "{PLAYLIST_SELECT} WHERE p.name LIKE ? ESCAPE '\\' \
             GROUP BY p.id ORDER BY p.name COLLATE NOCASE ASC, p.id ASC LIMIT ?"
        ))
        .bind(pattern.to_owned())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut playlists = Vec::with_capacity(rows.len());
        for row in &rows {
            playlists.push(playlist_from_row(row)?);
        }
        Ok(playlists)
    }
}

#[async_trait::async_trait]
impl SearchRepository for SqliteSearchRepository {
    /// `limit` applies to each bucket, not to their sum.
    async fn all(&self, q: &str, limit: i64) -> CoreResult<SearchResults> {
        let query = q.trim();
        if query.is_empty() {
            return Ok(SearchResults::default());
        }
        let limit = clamp_limit(limit);
        let pattern = like_contains(query);

        Ok(SearchResults {
            tracks: self.matching_tracks(query, limit).await?,
            albums: self.matching_albums(&pattern, limit).await?,
            artists: self.matching_artists(&pattern, limit).await?,
            playlists: self.matching_playlists(&pattern, limit).await?,
        })
    }

    async fn tracks(&self, q: &str, limit: i64) -> CoreResult<Vec<Track>> {
        self.matching_tracks(q.trim(), limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteSearchRepository;
    use crate::domain::ScannedTrack;
    use crate::infrastructure::repositories::{
        PlaylistRepository, SearchRepository, TrackRepository,
    };
    use crate::infrastructure::sqlite::playlist_repo::SqlitePlaylistRepository;
    use crate::infrastructure::sqlite::sql::test_support::{pool, scanned};
    use crate::infrastructure::sqlite::track_repo::SqliteTrackRepository;

    fn tagged(uri: &str, title: &str, artist: &str, album: &str, genre: &str) -> ScannedTrack {
        ScannedTrack {
            artist: Some(artist.to_owned()),
            album: Some(album.to_owned()),
            album_artist: Some(artist.to_owned()),
            genre: Some(genre.to_owned()),
            ..scanned(uri, title)
        }
    }

    async fn seeded() -> (SqliteSearchRepository, SqliteTrackRepository) {
        let db = pool().await;
        let tracks = SqliteTrackRepository::new(db.clone());
        tracks
            .upsert_many(&[
                tagged(
                    "content://1",
                    "Wish You Were Here",
                    "Pink Floyd",
                    "Wish You Were Here",
                    "Rock",
                ),
                tagged(
                    "content://2",
                    "Money",
                    "Pink Floyd",
                    "The Dark Side",
                    "Rock",
                ),
                tagged("content://3", "Аквариум", "Аквариум", "Синий альбом", "Рок"),
                tagged("content://4", "50% Off", "Discount", "Sale", "Pop"),
            ])
            .await
            .expect("library scan");

        let playlists = SqlitePlaylistRepository::new(db.clone());
        playlists.create("Floyd forever").await.expect("playlist");

        (SqliteSearchRepository::new(db), tracks)
    }

    #[tokio::test]
    async fn tracks_match_title_artist_and_album() {
        let (search, _tracks) = seeded().await;

        let by_artist = search.tracks("pink floyd", 10).await.expect("by artist");
        assert_eq!(by_artist.len(), 2);

        let by_title = search.tracks("money", 10).await.expect("by title");
        assert_eq!(by_title.len(), 1);
        assert_eq!(by_title[0].title, "Money");

        let by_album = search.tracks("dark side", 10).await.expect("by album");
        assert_eq!(by_album.len(), 1);

        let cyrillic = search.tracks("аквар", 10).await.expect("cyrillic prefix");
        assert_eq!(cyrillic.len(), 1);
    }

    #[tokio::test]
    async fn queries_are_prefix_matches() {
        let (search, _tracks) = seeded().await;
        assert_eq!(search.tracks("flo", 10).await.expect("prefix").len(), 2);
        assert_eq!(search.tracks("mon", 10).await.expect("prefix").len(), 1);
        assert!(search.tracks("zzz", 10).await.expect("no match").is_empty());
    }

    #[tokio::test]
    async fn fts_syntax_in_the_query_cannot_break_it() {
        let (search, _tracks) = seeded().await;

        // Quotes, stars and dashes are stripped, so these are all plain "money".
        for query in ["\"money\"", "money*", "-money", "(money)", " money "] {
            let found = search
                .tracks(query, 10)
                .await
                .unwrap_or_else(|error| panic!("query {query} failed: {error}"));
            assert_eq!(found.len(), 1, "query {query}");
        }

        // Operator words survive as ordinary tokens and are ANDed: no match,
        // but no syntax error either.
        for query in ["money OR", "NEAR/2 money", "money AND rock"] {
            let found = search
                .tracks(query, 10)
                .await
                .unwrap_or_else(|error| panic!("query {query} failed: {error}"));
            assert!(found.is_empty(), "query {query} matched {found:?}");
        }

        assert!(search.tracks("", 10).await.expect("blank").is_empty());
        assert!(search
            .tracks("  ***  ", 10)
            .await
            .expect("noise")
            .is_empty());
    }

    #[tokio::test]
    async fn all_fills_every_bucket() {
        let (search, _tracks) = seeded().await;

        let results = search.all("floyd", 10).await.expect("search");
        assert_eq!(results.tracks.len(), 2);
        assert_eq!(results.artists.len(), 1);
        assert_eq!(results.artists[0].name, "Pink Floyd");
        assert_eq!(results.playlists.len(), 1);
        assert_eq!(results.playlists[0].name, "Floyd forever");
        assert!(results.albums.is_empty());

        let albums = search.all("wish", 10).await.expect("search");
        assert_eq!(albums.albums.len(), 1);
        assert_eq!(albums.albums[0].title, "Wish You Were Here");

        assert!(search.all("   ", 10).await.expect("blank").is_empty());
    }

    #[tokio::test]
    async fn like_wildcards_are_literal() {
        let (search, _tracks) = seeded().await;

        let percent = search.all("50%", 10).await.expect("percent");
        assert_eq!(percent.tracks.len(), 1, "the FTS side still matches 50");

        let underscore = search.all("_", 10).await.expect("underscore");
        assert!(
            underscore.albums.is_empty() && underscore.artists.is_empty(),
            "_ must not behave as a wildcard"
        );
    }

    #[tokio::test]
    async fn the_index_follows_edits_and_deletions() {
        let (search, tracks) = seeded().await;

        let mut renamed = tagged(
            "content://2",
            "Us and Them",
            "Pink Floyd",
            "The Dark Side",
            "Rock",
        );
        renamed.duration_ms = 400_000;
        tracks.upsert_many(&[renamed]).await.expect("rescan");

        assert!(search
            .tracks("money", 10)
            .await
            .expect("old title")
            .is_empty());
        assert_eq!(
            search.tracks("us and them", 10).await.expect("new").len(),
            1
        );

        tracks
            .delete_missing(&["content://1".to_owned()])
            .await
            .expect("cleanup");
        assert!(search
            .tracks("us and them", 10)
            .await
            .expect("deleted")
            .is_empty());
        assert_eq!(search.tracks("wish", 10).await.expect("kept").len(), 1);
    }
}
