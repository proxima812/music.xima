//! `playlist_*` and `smart_playlist_*` (CONTRACTS §5, playlist.rs).

use tauri::State;

use crate::domain::{Playlist, SmartPlaylist, SmartPlaylistDraft, Track};
use crate::error::CoreResult;
use crate::state::AppState;

#[tauri::command]
pub async fn playlist_list(state: State<'_, AppState>) -> CoreResult<Vec<Playlist>> {
    state.playlists.list().await
}

#[tauri::command]
pub async fn playlist_get(state: State<'_, AppState>, id: i64) -> CoreResult<Playlist> {
    state.playlists.get(id).await
}

#[tauri::command]
pub async fn playlist_create(state: State<'_, AppState>, name: String) -> CoreResult<Playlist> {
    state.playlists.create(&name).await
}

#[tauri::command]
pub async fn playlist_rename(state: State<'_, AppState>, id: i64, name: String) -> CoreResult<()> {
    state.playlists.rename(id, &name).await
}

#[tauri::command]
pub async fn playlist_delete(state: State<'_, AppState>, id: i64) -> CoreResult<()> {
    state.playlists.delete(id).await
}

#[tauri::command]
pub async fn playlist_tracks(state: State<'_, AppState>, id: i64) -> CoreResult<Vec<Track>> {
    state.playlists.tracks(id).await
}

#[tauri::command]
pub async fn playlist_add_tracks(
    state: State<'_, AppState>,
    id: i64,
    track_ids: Vec<i64>,
) -> CoreResult<()> {
    state.playlists.add_tracks(id, &track_ids).await
}

#[tauri::command]
pub async fn playlist_remove_at(
    state: State<'_, AppState>,
    id: i64,
    position: i64,
) -> CoreResult<()> {
    state.playlists.remove_at(id, position).await
}

#[tauri::command]
pub async fn playlist_reorder(
    state: State<'_, AppState>,
    id: i64,
    from: i64,
    to: i64,
) -> CoreResult<()> {
    state.playlists.reorder(id, from, to).await
}

#[tauri::command]
pub async fn smart_playlist_list(state: State<'_, AppState>) -> CoreResult<Vec<SmartPlaylist>> {
    state.playlists.smart_list().await
}

#[tauri::command]
pub async fn smart_playlist_get(state: State<'_, AppState>, id: i64) -> CoreResult<SmartPlaylist> {
    state.playlists.smart_get(id).await
}

#[tauri::command]
pub async fn smart_playlist_create(
    state: State<'_, AppState>,
    draft: SmartPlaylistDraft,
) -> CoreResult<SmartPlaylist> {
    state.playlists.smart_create(&draft).await
}

#[tauri::command]
pub async fn smart_playlist_update(
    state: State<'_, AppState>,
    id: i64,
    draft: SmartPlaylistDraft,
) -> CoreResult<SmartPlaylist> {
    state.playlists.smart_update(id, &draft).await
}

#[tauri::command]
pub async fn smart_playlist_delete(state: State<'_, AppState>, id: i64) -> CoreResult<()> {
    state.playlists.smart_delete(id).await
}

#[tauri::command]
pub async fn smart_playlist_resolve(state: State<'_, AppState>, id: i64) -> CoreResult<Vec<Track>> {
    state.playlists.smart_resolve(id).await
}

#[tauri::command]
pub async fn smart_playlist_preview(
    state: State<'_, AppState>,
    draft: SmartPlaylistDraft,
) -> CoreResult<Vec<Track>> {
    state.playlists.smart_preview(&draft).await
}
