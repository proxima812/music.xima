//! Managed application state.
//!
//! One `Arc` per application service, handed to the commands through
//! `tauri::State`. Nothing is stored here that a service does not already own:
//! the database pool lives inside the repositories, playback state lives in the
//! native player (CONTRACTS §1.5).

use std::sync::Arc;

use crate::application::{
    HistoryService, LibraryService, PlayerService, PlaylistService, ScanService, SearchService,
    StatisticsService,
};

pub struct AppState {
    pub library: Arc<LibraryService>,
    pub search: Arc<SearchService>,
    pub playlists: Arc<PlaylistService>,
    pub history: Arc<HistoryService>,
    pub statistics: Arc<StatisticsService>,
    pub player: Arc<PlayerService>,
    pub scan: Arc<ScanService>,
}

impl AppState {
    /// Assembled once in `lib.rs`; the services are shared with the event
    /// bridge, which is why they arrive as `Arc`s rather than by value.
    pub fn new(
        library: Arc<LibraryService>,
        search: Arc<SearchService>,
        playlists: Arc<PlaylistService>,
        history: Arc<HistoryService>,
        statistics: Arc<StatisticsService>,
        player: Arc<PlayerService>,
        scan: Arc<ScanService>,
    ) -> Self {
        Self {
            library,
            search,
            playlists,
            history,
            statistics,
            player,
            scan,
        }
    }
}
