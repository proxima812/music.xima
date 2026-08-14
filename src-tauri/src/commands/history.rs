//! `history_*` (CONTRACTS §5).
//!
//! Playback history is written by Rust when the native player reports
//! `completed` (see `infrastructure::android::events`). `history_record` stays
//! exposed for manual scenarios and tests.

use tauri::State;

use crate::domain::Track;
use crate::error::CoreResult;
use crate::state::AppState;

#[tauri::command]
pub async fn history_record(
    state: State<'_, AppState>,
    track_id: i64,
    played_at: i64,
    duration_played_ms: i64,
) -> CoreResult<()> {
    state
        .history
        .record(track_id, played_at, duration_played_ms)
        .await
}

#[tauri::command]
pub async fn history_recent(state: State<'_, AppState>, limit: i64) -> CoreResult<Vec<Track>> {
    state.history.recent(limit).await
}
