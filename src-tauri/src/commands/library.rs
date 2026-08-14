//! `library_*` and `artwork_uri` (CONTRACTS §5, library.rs).

use tauri::{AppHandle, State};

use crate::domain::{
    Album, Artist, Folder, Genre, LibraryStats, Page, ScanMode, ScanResult, ScanStatus, Track,
    TrackQuery,
};
use crate::error::CoreResult;
use crate::infrastructure::android::emit_library_changed;
use crate::state::AppState;

#[tauri::command]
pub async fn library_stats(state: State<'_, AppState>) -> CoreResult<LibraryStats> {
    state.library.stats().await
}

#[tauri::command]
pub async fn library_tracks(
    state: State<'_, AppState>,
    query: TrackQuery,
) -> CoreResult<Page<Track>> {
    state.library.tracks(&query).await
}

#[tauri::command]
pub async fn library_track(state: State<'_, AppState>, id: i64) -> CoreResult<Track> {
    state.library.track(id).await
}

#[tauri::command]
pub async fn library_tracks_by_ids(
    state: State<'_, AppState>,
    ids: Vec<i64>,
) -> CoreResult<Vec<Track>> {
    state.library.tracks_by_ids(&ids).await
}

#[tauri::command]
pub async fn library_recently_added(
    state: State<'_, AppState>,
    limit: i64,
) -> CoreResult<Vec<Track>> {
    state.library.recently_added(limit).await
}

#[tauri::command]
pub async fn library_albums(
    state: State<'_, AppState>,
    offset: i64,
    limit: i64,
) -> CoreResult<Page<Album>> {
    state.library.albums(offset, limit).await
}

#[tauri::command]
pub async fn library_album(state: State<'_, AppState>, id: i64) -> CoreResult<Album> {
    state.library.album(id).await
}

#[tauri::command]
pub async fn library_album_tracks(
    state: State<'_, AppState>,
    album_id: i64,
) -> CoreResult<Vec<Track>> {
    state.library.album_tracks(album_id).await
}

#[tauri::command]
pub async fn library_artists(
    state: State<'_, AppState>,
    offset: i64,
    limit: i64,
) -> CoreResult<Page<Artist>> {
    state.library.artists(offset, limit).await
}

#[tauri::command]
pub async fn library_artist(state: State<'_, AppState>, id: i64) -> CoreResult<Artist> {
    state.library.artist(id).await
}

#[tauri::command]
pub async fn library_artist_albums(
    state: State<'_, AppState>,
    artist_id: i64,
) -> CoreResult<Vec<Album>> {
    state.library.artist_albums(artist_id).await
}

#[tauri::command]
pub async fn library_artist_tracks(
    state: State<'_, AppState>,
    artist_id: i64,
) -> CoreResult<Vec<Track>> {
    state.library.artist_tracks(artist_id).await
}

#[tauri::command]
pub async fn library_genres(state: State<'_, AppState>) -> CoreResult<Vec<Genre>> {
    state.library.genres().await
}

#[tauri::command]
pub async fn library_folders(
    state: State<'_, AppState>,
    parent: Option<String>,
) -> CoreResult<Vec<Folder>> {
    state.library.folders(parent.as_deref()).await
}

/// Progress arrives as `library:scan-progress` while this runs; the finished
/// scan announces itself with `library:changed` (CONTRACTS §6).
#[tauri::command]
pub async fn library_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    roots: Vec<String>,
    mode: ScanMode,
) -> CoreResult<ScanResult> {
    let result = state.scan.scan(&roots, mode).await?;
    emit_library_changed(&app, "scan");
    Ok(result)
}

#[tauri::command]
pub async fn library_scan_status(state: State<'_, AppState>) -> CoreResult<ScanStatus> {
    Ok(state.scan.status())
}

/// Opens the SAF picker. `None` when the user dismissed it.
#[tauri::command]
pub async fn library_pick_folder(state: State<'_, AppState>) -> CoreResult<Option<String>> {
    state.scan.pick_folder().await
}

#[tauri::command]
pub async fn library_roots(state: State<'_, AppState>) -> CoreResult<Vec<String>> {
    state.scan.roots().await
}

#[tauri::command]
pub async fn library_remove_root(
    app: AppHandle,
    state: State<'_, AppState>,
    uri: String,
) -> CoreResult<()> {
    state.scan.remove_root(&uri).await?;
    emit_library_changed(&app, "root-removed");
    Ok(())
}

#[tauri::command]
pub async fn artwork_uri(
    state: State<'_, AppState>,
    cover_key: String,
) -> CoreResult<Option<String>> {
    state.library.artwork_uri(&cover_key).await
}
