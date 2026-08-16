//! Application services: orchestration between the command layer and the
//! repositories.
//!
//! No SQL, no Android, no Tauri lives here. Everything the outside world
//! provides arrives as an injected trait object: repositories from
//! `infrastructure::repositories`, the native player as [`PlayerPort`], the
//! native scanner as [`ScannerPort`], artwork resolution as [`ArtworkPort`].

pub mod history_service;
pub mod library_service;
pub mod player_service;
pub mod playlist_service;
pub mod scan_service;
pub mod search_service;
pub mod statistics_service;
pub mod track_removal_service;

pub use history_service::HistoryService;
pub use library_service::LibraryService;
pub use player_service::{
    PlayerPort, PlayerService, MAX_CROSSFADE_MS, MAX_SPEED, MAX_VOLUME, MIN_SPEED, MIN_VOLUME,
};
pub use playlist_service::PlaylistService;
pub use scan_service::{ProgressCallback, ScanBatch, ScanService, ScannerPort, MEDIA_STORE_ROOT};
pub use search_service::SearchService;
pub use statistics_service::StatisticsService;
pub use track_removal_service::{
    RecoveryFailure, RecoveryReport, TrackFilePort, TrackRemovalService,
};

use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::MAX_PAGE_LIMIT;
use crate::error::{CoreError, CoreResult};

/// Wall clock, injected so services stay testable. Unix milliseconds, UTC.
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> i64;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        now_ms()
    }
}

/// Current time in Unix milliseconds. Saturates instead of panicking when the
/// device clock sits outside the representable range.
pub fn now_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(elapsed) => i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX),
        Err(before_epoch) => i64::try_from(before_epoch.duration().as_millis())
            .map(|millis| -millis)
            .unwrap_or(i64::MIN),
    }
}

/// Resolves a `coverKey` (see `Track::cover_key`) to a `file://` / `content://`
/// URI of the cached artwork. Implemented by the adapter in `infrastructure`,
/// because it touches the app cache directory.
#[async_trait::async_trait]
pub trait ArtworkPort: Send + Sync {
    async fn artwork_uri(&self, cover_key: &str) -> CoreResult<Option<String>>;
}

/// Pagination is zero-based; a negative offset is a frontend bug, not a
/// silently corrected value.
pub fn validated_offset(offset: i64) -> CoreResult<i64> {
    if offset < 0 {
        return Err(CoreError::invalid_input("offset must not be negative"));
    }
    Ok(offset)
}

/// Rejects non-positive limits and caps everything at [`MAX_PAGE_LIMIT`], so a
/// single IPC response can never carry an unbounded number of rows.
pub fn validated_limit(limit: i64) -> CoreResult<i64> {
    if limit <= 0 {
        return Err(CoreError::invalid_input("limit must be positive"));
    }
    Ok(limit.min(MAX_PAGE_LIMIT))
}

/// Row ids handed over by the frontend are always positive.
pub fn validated_id(entity: &str, id: i64) -> CoreResult<i64> {
    if id <= 0 {
        return Err(CoreError::invalid_input(format!(
            "{entity} id must be positive"
        )));
    }
    Ok(id)
}

/// Same rule as [`validated_id`], applied to a whole list.
pub fn validated_track_ids(ids: &[i64]) -> CoreResult<()> {
    if ids.iter().any(|id| *id <= 0) {
        return Err(CoreError::invalid_input("track ids must be positive"));
    }
    Ok(())
}

/// Positions and indexes are zero-based.
pub fn validated_index(name: &str, index: i64) -> CoreResult<()> {
    if index < 0 {
        return Err(CoreError::invalid_input(format!(
            "{name} must not be negative"
        )));
    }
    Ok(())
}

/// Row counts come back from repositories as `u64`; the DTOs are `i64`.
pub(crate) fn count_as_i64(count: u64) -> i64 {
    i64::try_from(count).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        now_ms, validated_id, validated_index, validated_limit, validated_offset,
        validated_track_ids,
    };
    use crate::domain::MAX_PAGE_LIMIT;

    #[test]
    fn offsets_must_not_be_negative() {
        assert_eq!(validated_offset(0).expect("zero is valid"), 0);
        assert_eq!(validated_offset(120).expect("positive is valid"), 120);
        let error = validated_offset(-1).expect_err("negative offset is rejected");
        assert_eq!(error.code(), "INVALID_INPUT");
    }

    #[test]
    fn limits_are_positive_and_capped() {
        assert_eq!(validated_limit(25).expect("in range"), 25);
        assert_eq!(
            validated_limit(10_000).expect("capped, not rejected"),
            MAX_PAGE_LIMIT
        );
        assert_eq!(
            validated_limit(0).expect_err("zero limit").code(),
            "INVALID_INPUT"
        );
        assert_eq!(
            validated_limit(-5).expect_err("negative limit").code(),
            "INVALID_INPUT"
        );
    }

    #[test]
    fn ids_must_be_positive() {
        assert_eq!(validated_id("track", 7).expect("valid id"), 7);
        let error = validated_id("album", 0).expect_err("zero id is rejected");
        assert_eq!(error.code(), "INVALID_INPUT");
        assert!(error.to_string().contains("album"));

        assert!(validated_track_ids(&[1, 2, 3]).is_ok());
        assert!(validated_track_ids(&[]).is_ok());
        assert!(validated_track_ids(&[1, 0]).is_err());
        assert!(validated_track_ids(&[-4]).is_err());
    }

    #[test]
    fn indexes_must_not_be_negative() {
        assert!(validated_index("position", 0).is_ok());
        assert_eq!(
            validated_index("position", -1)
                .expect_err("negative position")
                .code(),
            "INVALID_INPUT"
        );
    }

    #[test]
    fn clock_is_in_millisecond_range() {
        // Sanity: 2020-01-01 < now < 2100-01-01, i.e. milliseconds, not seconds.
        let now = now_ms();
        assert!(now > 1_577_836_800_000, "clock looks like seconds: {now}");
        assert!(now < 4_102_444_800_000, "clock is too far ahead: {now}");
    }
}

#[cfg(test)]
pub(crate) mod testing {
    //! Shared test doubles. Repository fakes stay next to the service they
    //! exercise; only the pieces every service needs live here.

    use super::{ArtworkPort, Clock};
    use crate::domain::Track;
    use crate::error::CoreResult;

    pub fn track(id: i64) -> Track {
        Track {
            id,
            uri: format!("content://media/external/audio/media/{id}"),
            title: format!("Track {id}"),
            artist_id: Some(1),
            artist_name: Some("Artist".to_owned()),
            album_id: Some(2),
            album_title: Some("Album".to_owned()),
            duration_ms: 200_000 + id,
            track_number: Some(1),
            disc_number: Some(1),
            year: Some(2020),
            genre: Some("Rock".to_owned()),
            bitrate: Some(320_000),
            sample_rate: Some(44_100),
            size: 8_000_000,
            format: Some("MP3".to_owned()),
            cover_key: Some(format!("cover-{id}")),
            folder: Some("Music".to_owned()),
            date_added: 1_700_000_000_000,
            last_modified: 1_700_000_000_000,
            is_favorite: false,
            play_count: 0,
            last_played_at: None,
        }
    }

    pub struct FixedClock(pub i64);

    impl Clock for FixedClock {
        fn now_ms(&self) -> i64 {
            self.0
        }
    }

    /// Resolves every cover key to a cache URI.
    pub struct CacheArtwork;

    #[async_trait::async_trait]
    impl ArtworkPort for CacheArtwork {
        async fn artwork_uri(&self, cover_key: &str) -> CoreResult<Option<String>> {
            Ok(Some(format!("file:///cache/{cover_key}.jpg")))
        }
    }
}
