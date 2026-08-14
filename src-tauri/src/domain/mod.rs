//! Domain types and pure rules (CONTRACTS §1, §1.6, §3).
//!
//! Nothing here knows about SQLite, Tauri or Android. Every DTO that crosses
//! the IPC boundary lives in this module and is `camelCase` on the wire.

pub mod album;
pub mod artist;
pub mod playlist;
pub mod query;
pub mod queue;
pub mod scan;
pub mod smart;
pub mod sort;
pub mod stats;
pub mod taxonomy;
pub mod track;
pub mod track_removal;

pub use album::Album;
pub use artist::Artist;
pub use playlist::{reorder_target, sanitize_playlist_name, Playlist, MAX_PLAYLIST_NAME_LEN};
pub use query::{
    clamp_limit, clamp_offset, LibraryStats, Page, SearchResults, TrackQuery, TrackSort,
    TrackSortField, DEFAULT_PAGE_LIMIT, MAX_PAGE_LIMIT,
};
pub use queue::{PlaybackState, PlaybackStatus, RepeatMode};
pub use scan::{
    format_from_mime, ScanMode, ScanResult, ScanStatus, ScannedTrack, PHASE_CLEANUP, PHASE_DONE,
    PHASE_ENUMERATING, PHASE_IDLE, PHASE_READING, PHASE_WRITING,
};
pub use smart::{NumOp, SmartPlaylist, SmartPlaylistDraft, SmartRule, SmartSort};
pub use sort::normalize_sort_key;
pub use stats::{RankedTrack, StatsRange, MS_PER_DAY};
pub use taxonomy::{folder_name, folder_parent, Folder, Genre, FOLDER_SEPARATOR};
pub use track::Track;
pub use track_removal::{DeleteTrackResult, FileDeleteOutcome, HiddenTrack, PendingDeletion};
