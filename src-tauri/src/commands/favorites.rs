//! `favorite_*` (CONTRACTS §5, favorites.rs).

use tauri::State;

use crate::domain::Track;
use crate::error::CoreResult;
use crate::state::AppState;

/// Returns the new favorite state of the track.
#[tauri::command]
pub async fn favorite_toggle(state: State<'_, AppState>, track_id: i64) -> CoreResult<bool> {
    state.library.toggle_favorite(track_id).await
}

#[tauri::command]
pub async fn favorite_list(state: State<'_, AppState>) -> CoreResult<Vec<Track>> {
    state.library.favorites().await
}
