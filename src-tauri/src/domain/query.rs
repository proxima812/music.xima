//! Query, pagination and library-wide aggregate types (CONTRACTS §1.6).

use serde::{Deserialize, Serialize};

use crate::domain::album::Album;
use crate::domain::artist::Artist;
use crate::domain::playlist::Playlist;
use crate::domain::track::Track;

/// Page size used when the caller does not care.
pub const DEFAULT_PAGE_LIMIT: i64 = 100;
/// Hard ceiling: a single IPC response never carries more rows than this.
pub const MAX_PAGE_LIMIT: i64 = 500;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrackSort {
    #[default]
    TitleAsc,
    TitleDesc,
    ArtistAsc,
    AlbumAsc,
    DateAddedDesc,
    DateAddedAsc,
    DurationAsc,
    DurationDesc,
    PlayCountDesc,
    LastPlayedDesc,
    YearDesc,
}

/// What a [`TrackSort`] orders by, with the direction factored out. Lets the
/// SQL layer pick a column without re-matching eleven variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackSortField {
    Title,
    Artist,
    Album,
    DateAdded,
    Duration,
    PlayCount,
    LastPlayed,
    Year,
}

impl TrackSort {
    pub const fn field(self) -> TrackSortField {
        match self {
            Self::TitleAsc | Self::TitleDesc => TrackSortField::Title,
            Self::ArtistAsc => TrackSortField::Artist,
            Self::AlbumAsc => TrackSortField::Album,
            Self::DateAddedDesc | Self::DateAddedAsc => TrackSortField::DateAdded,
            Self::DurationAsc | Self::DurationDesc => TrackSortField::Duration,
            Self::PlayCountDesc => TrackSortField::PlayCount,
            Self::LastPlayedDesc => TrackSortField::LastPlayed,
            Self::YearDesc => TrackSortField::Year,
        }
    }

    pub const fn descending(self) -> bool {
        matches!(
            self,
            Self::TitleDesc
                | Self::DateAddedDesc
                | Self::DurationDesc
                | Self::PlayCountDesc
                | Self::LastPlayedDesc
                | Self::YearDesc
        )
    }

    /// Stable token, identical to the JSON representation. Used to persist the
    /// sort in settings and in `smart_playlists.sort`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TitleAsc => "TITLE_ASC",
            Self::TitleDesc => "TITLE_DESC",
            Self::ArtistAsc => "ARTIST_ASC",
            Self::AlbumAsc => "ALBUM_ASC",
            Self::DateAddedDesc => "DATE_ADDED_DESC",
            Self::DateAddedAsc => "DATE_ADDED_ASC",
            Self::DurationAsc => "DURATION_ASC",
            Self::DurationDesc => "DURATION_DESC",
            Self::PlayCountDesc => "PLAY_COUNT_DESC",
            Self::LastPlayedDesc => "LAST_PLAYED_DESC",
            Self::YearDesc => "YEAR_DESC",
        }
    }

    pub fn from_token(token: &str) -> Option<Self> {
        let all = [
            Self::TitleAsc,
            Self::TitleDesc,
            Self::ArtistAsc,
            Self::AlbumAsc,
            Self::DateAddedDesc,
            Self::DateAddedAsc,
            Self::DurationAsc,
            Self::DurationDesc,
            Self::PlayCountDesc,
            Self::LastPlayedDesc,
            Self::YearDesc,
        ];
        all.into_iter().find(|sort| sort.as_str() == token)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackQuery {
    pub sort: TrackSort,
    pub offset: i64,
    pub limit: i64,
    pub artist_id: Option<i64>,
    pub album_id: Option<i64>,
    pub genre: Option<String>,
    pub folder: Option<String>,
    pub favorites_only: bool,
}

impl Default for TrackQuery {
    fn default() -> Self {
        Self {
            sort: TrackSort::default(),
            offset: 0,
            limit: DEFAULT_PAGE_LIMIT,
            artist_id: None,
            album_id: None,
            genre: None,
            folder: None,
            favorites_only: false,
        }
    }
}

impl TrackQuery {
    /// Query with untrusted numbers clamped and blank filters dropped. The SQL
    /// layer works on this, never on the raw IPC payload.
    pub fn normalized(&self) -> Self {
        Self {
            sort: self.sort,
            offset: clamp_offset(self.offset),
            limit: clamp_limit(self.limit),
            artist_id: self.artist_id,
            album_id: self.album_id,
            genre: normalize_filter(self.genre.as_deref()),
            folder: normalize_filter(self.folder.as_deref()),
            favorites_only: self.favorites_only,
        }
    }

    pub fn has_filters(&self) -> bool {
        self.artist_id.is_some()
            || self.album_id.is_some()
            || self.genre.is_some()
            || self.folder.is_some()
            || self.favorites_only
    }
}

/// Negative offsets become 0.
pub const fn clamp_offset(offset: i64) -> i64 {
    if offset < 0 {
        0
    } else {
        offset
    }
}

/// Non-positive limits fall back to the default, oversized ones are capped.
pub const fn clamp_limit(limit: i64) -> i64 {
    if limit <= 0 {
        DEFAULT_PAGE_LIMIT
    } else if limit > MAX_PAGE_LIMIT {
        MAX_PAGE_LIMIT
    } else {
        limit
    }
}

fn normalize_filter(raw: Option<&str>) -> Option<String> {
    raw.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: i64, offset: i64, limit: i64) -> Self {
        Self {
            items,
            total,
            offset,
            limit,
        }
    }

    pub fn empty(offset: i64, limit: i64) -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            offset,
            limit,
        }
    }

    /// True when `offset + items.len()` has not reached `total` yet.
    pub fn has_more(&self) -> bool {
        let seen = self.offset.saturating_add(self.items.len() as i64);
        seen < self.total
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub tracks: i64,
    pub albums: i64,
    pub artists: i64,
    pub playlists: i64,
    pub genres: i64,
    pub total_duration_ms: i64,
    pub total_size: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
}

impl SearchResults {
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
            && self.albums.is_empty()
            && self.artists.is_empty()
            && self.playlists.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        clamp_limit, clamp_offset, LibraryStats, Page, SearchResults, TrackQuery, TrackSort,
        TrackSortField, DEFAULT_PAGE_LIMIT, MAX_PAGE_LIMIT,
    };

    #[test]
    fn sort_tokens_match_json() {
        for sort in [
            TrackSort::TitleAsc,
            TrackSort::TitleDesc,
            TrackSort::ArtistAsc,
            TrackSort::AlbumAsc,
            TrackSort::DateAddedDesc,
            TrackSort::DateAddedAsc,
            TrackSort::DurationAsc,
            TrackSort::DurationDesc,
            TrackSort::PlayCountDesc,
            TrackSort::LastPlayedDesc,
            TrackSort::YearDesc,
        ] {
            let json = serde_json::to_string(&sort).expect("sort serializes");
            assert_eq!(json, format!("\"{}\"", sort.as_str()));
            assert_eq!(TrackSort::from_token(sort.as_str()), Some(sort));
        }
        assert_eq!(TrackSort::from_token("NOPE"), None);
    }

    #[test]
    fn sort_exposes_field_and_direction() {
        assert_eq!(TrackSort::TitleDesc.field(), TrackSortField::Title);
        assert!(TrackSort::TitleDesc.descending());
        assert!(!TrackSort::TitleAsc.descending());
        assert_eq!(
            TrackSort::LastPlayedDesc.field(),
            TrackSortField::LastPlayed
        );
        assert!(!TrackSort::DateAddedAsc.descending());
    }

    #[test]
    fn clamps_pagination() {
        assert_eq!(clamp_offset(-5), 0);
        assert_eq!(clamp_offset(12), 12);
        assert_eq!(clamp_limit(0), DEFAULT_PAGE_LIMIT);
        assert_eq!(clamp_limit(-3), DEFAULT_PAGE_LIMIT);
        assert_eq!(clamp_limit(10_000), MAX_PAGE_LIMIT);
        assert_eq!(clamp_limit(25), 25);
    }

    #[test]
    fn normalizes_query() {
        let raw = TrackQuery {
            sort: TrackSort::YearDesc,
            offset: -10,
            limit: 100_000,
            artist_id: None,
            album_id: Some(4),
            genre: Some("   ".to_owned()),
            folder: Some(" Music/Rock ".to_owned()),
            favorites_only: true,
        };
        let normalized = raw.normalized();
        assert_eq!(normalized.offset, 0);
        assert_eq!(normalized.limit, MAX_PAGE_LIMIT);
        assert_eq!(normalized.genre, None);
        assert_eq!(normalized.folder.as_deref(), Some("Music/Rock"));
        assert_eq!(normalized.sort, TrackSort::YearDesc);
        assert!(normalized.has_filters());
    }

    #[test]
    fn default_query_is_first_page_by_title() {
        let query = TrackQuery::default();
        assert_eq!(query.sort, TrackSort::TitleAsc);
        assert_eq!(query.offset, 0);
        assert_eq!(query.limit, DEFAULT_PAGE_LIMIT);
        assert!(!query.has_filters());
    }

    #[test]
    fn query_serializes_in_camel_case() {
        let value = serde_json::to_value(TrackQuery::default()).expect("query serializes");
        assert!(value.get("favoritesOnly").is_some());
        assert!(value.get("artistId").is_some());
        assert_eq!(
            value.get("sort").and_then(|v| v.as_str()),
            Some("TITLE_ASC")
        );
    }

    #[test]
    fn page_reports_more_items() {
        let page = Page::new(vec![1_i64, 2, 3], 10, 0, 3);
        assert!(page.has_more());
        let last = Page::new(vec![9_i64], 10, 9, 3);
        assert!(!last.has_more());
        let empty: Page<i64> = Page::empty(0, 50);
        assert!(!empty.has_more());
        assert_eq!(empty.limit, 50);
    }

    #[test]
    fn page_round_trips_through_json() {
        let page = Page::new(vec!["a".to_owned()], 1, 0, 20);
        let json = serde_json::to_string(&page).expect("page serializes");
        assert!(json.contains("\"items\""));
        let back: Page<String> = serde_json::from_str(&json).expect("page deserializes");
        assert_eq!(back, page);
    }

    #[test]
    fn aggregates_have_camel_case_and_defaults() {
        let value = serde_json::to_value(LibraryStats::default()).expect("stats serialize");
        assert_eq!(
            value.get("totalDurationMs").and_then(|v| v.as_i64()),
            Some(0)
        );
        assert_eq!(value.get("totalSize").and_then(|v| v.as_i64()), Some(0));
        assert!(SearchResults::default().is_empty());
    }
}
