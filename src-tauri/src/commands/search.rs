//! `search_*` (CONTRACTS §5, search.rs).

use tauri::State;

use crate::domain::{SearchResults, Track};
use crate::error::CoreResult;
use crate::state::AppState;

#[tauri::command]
pub async fn search_all(
    state: State<'_, AppState>,
    q: String,
    limit: i64,
) -> CoreResult<SearchResults> {
    state.search.all(&q, limit).await
}

#[tauri::command]
pub async fn search_tracks(
    state: State<'_, AppState>,
    q: String,
    limit: i64,
) -> CoreResult<Vec<Track>> {
    state.search.tracks(&q, limit).await
}
