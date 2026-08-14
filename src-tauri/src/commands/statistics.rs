//! `stats_*` (CONTRACTS §5, statistics.rs).

use tauri::State;

use crate::domain::{RankedTrack, StatsRange, Track};
use crate::error::CoreResult;
use crate::state::AppState;

#[tauri::command]
pub async fn stats_top_tracks(
    state: State<'_, AppState>,
    range: StatsRange,
    limit: i64,
) -> CoreResult<Vec<RankedTrack>> {
    state.statistics.top_tracks(range, limit).await
}

#[tauri::command]
pub async fn stats_never_played(state: State<'_, AppState>, limit: i64) -> CoreResult<Vec<Track>> {
    state.statistics.never_played(limit).await
}

#[tauri::command]
pub async fn stats_forgotten(
    state: State<'_, AppState>,
    days: i64,
    limit: i64,
) -> CoreResult<Vec<Track>> {
    state.statistics.forgotten(days, limit).await
}
