//! Managed application state.
//!
//! One `Arc` per application service, handed to the commands through
//! `tauri::State`. Nothing is stored here that a service does not already own:
//! the database pool lives inside the repositories, playback state lives in the
//! native player (CONTRACTS §1.5).

use std::sync::Arc;

use crate::application::{
    HistoryService, LibraryService, PlayerService, PlaylistService, ScanService, SearchService,
    StatisticsService, TrackRemovalService,
};

pub struct AppState {
    pub library: Arc<LibraryService>,
    pub search: Arc<SearchService>,
    pub playlists: Arc<PlaylistService>,
    pub history: Arc<HistoryService>,
    pub statistics: Arc<StatisticsService>,
    pub player: Arc<PlayerService>,
    pub scan: Arc<ScanService>,
    pub track_removal: Arc<TrackRemovalService>,
}
