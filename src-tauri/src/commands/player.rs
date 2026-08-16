//! `player_*` (CONTRACTS §5, player.rs) — a proxy to the native plugin.
//!
//! Nothing here caches playback state: the native player is the source of
//! truth (CONTRACTS §1.5) and pushes changes through the `player:*` events.

use tauri::State;

use crate::domain::{PlaybackState, RepeatMode, Track};
use crate::error::CoreResult;
use crate::state::AppState;

#[tauri::command]
pub async fn player_state(state: State<'_, AppState>) -> CoreResult<PlaybackState> {
    state.player.state().await
}

#[tauri::command]
pub async fn player_set_queue(
    state: State<'_, AppState>,
    track_ids: Vec<i64>,
    start_index: i32,
    autoplay: bool,
) -> CoreResult<()> {
    state
        .player
        .set_queue(&track_ids, start_index, autoplay)
        .await
}

#[tauri::command]
pub async fn player_queue(state: State<'_, AppState>) -> CoreResult<Vec<Track>> {
    state.player.queue().await
}

#[tauri::command]
pub async fn player_play(state: State<'_, AppState>) -> CoreResult<()> {
    state.player.play().await
}

#[tauri::command]
pub async fn player_pause(state: State<'_, AppState>) -> CoreResult<()> {
    state.player.pause().await
}

#[tauri::command]
pub async fn player_toggle(state: State<'_, AppState>) -> CoreResult<()> {
    state.player.toggle().await
}

#[tauri::command]
pub async fn player_stop(state: State<'_, AppState>) -> CoreResult<()> {
    state.player.stop().await
}

#[tauri::command]
pub async fn player_next(state: State<'_, AppState>) -> CoreResult<()> {
    state.player.next().await
}

#[tauri::command]
pub async fn player_previous(state: State<'_, AppState>) -> CoreResult<()> {
    state.player.previous().await
}

#[tauri::command]
pub async fn player_seek(state: State<'_, AppState>, position_ms: i64) -> CoreResult<()> {
    state.player.seek(position_ms).await
}

#[tauri::command]
pub async fn player_skip_to(state: State<'_, AppState>, index: i32) -> CoreResult<()> {
    state.player.skip_to(index).await
}

#[tauri::command]
pub async fn player_set_shuffle(state: State<'_, AppState>, enabled: bool) -> CoreResult<()> {
    state.player.set_shuffle(enabled).await
}

#[tauri::command]
pub async fn player_set_repeat(state: State<'_, AppState>, mode: RepeatMode) -> CoreResult<()> {
    state.player.set_repeat(mode).await
}

#[tauri::command]
pub async fn player_set_volume(state: State<'_, AppState>, volume: f32) -> CoreResult<()> {
    state.player.set_volume(volume).await
}

#[tauri::command]
pub async fn player_set_speed(state: State<'_, AppState>, speed: f32) -> CoreResult<()> {
    state.player.set_speed(speed).await
}

#[tauri::command]
pub async fn player_set_crossfade(state: State<'_, AppState>, duration_ms: i64) -> CoreResult<()> {
    state.player.set_crossfade(duration_ms).await
}

#[tauri::command]
pub async fn player_add_next(state: State<'_, AppState>, track_ids: Vec<i64>) -> CoreResult<()> {
    state.player.add_next(&track_ids).await
}

#[tauri::command]
pub async fn player_add_to_queue(
    state: State<'_, AppState>,
    track_ids: Vec<i64>,
) -> CoreResult<()> {
    state.player.add_to_queue(&track_ids).await
}

#[tauri::command]
pub async fn player_remove_queue_item(state: State<'_, AppState>, index: i32) -> CoreResult<()> {
    state.player.remove_queue_item(index).await
}

#[tauri::command]
pub async fn player_move_queue_item(
    state: State<'_, AppState>,
    from: i32,
    to: i32,
) -> CoreResult<()> {
    state.player.move_queue_item(from, to).await
}

#[tauri::command]
pub async fn player_clear_queue(state: State<'_, AppState>) -> CoreResult<()> {
    state.player.clear_queue().await
}
