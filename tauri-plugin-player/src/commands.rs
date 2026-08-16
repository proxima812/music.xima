//! `plugin:player|<command>` bindings. The app frontend does not use them — it
//! goes through the core commands of CONTRACTS §5 — they exist for the TS
//! bindings and for poking the native player by hand while debugging.
//!
//! Command names are the snake_case list from `build.rs`; arguments arrive in
//! camelCase, as everywhere else in the IPC layer.

use tauri::{AppHandle, Runtime};

use crate::models::{
    DeleteFileResponse, PlaybackState, QueueIdsResponse, QueueItem, RepeatMode, ScanBatch,
    SetQueueRequest, TrackFileExistsResponse,
};
use crate::{PlayerExt, Result};

async fn blocking_plugin_call<T, F>(call: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(call)
        .await
        .map_err(|error| crate::Error::native(format!("player blocking task failed: {error}")))?
}

#[tauri::command]
pub(crate) async fn get_state<R: Runtime>(app: AppHandle<R>) -> Result<PlaybackState> {
    app.player().get_state()
}

#[tauri::command]
pub(crate) async fn get_queue_ids<R: Runtime>(app: AppHandle<R>) -> Result<QueueIdsResponse> {
    blocking_plugin_call(move || app.player().get_queue_ids()).await
}

#[tauri::command]
pub(crate) async fn set_queue<R: Runtime>(app: AppHandle<R>, req: SetQueueRequest) -> Result<()> {
    app.player().set_queue(req)
}

#[tauri::command]
pub(crate) async fn play<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.player().play()
}

#[tauri::command]
pub(crate) async fn pause<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.player().pause()
}

#[tauri::command]
pub(crate) async fn toggle<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.player().toggle()
}

#[tauri::command]
pub(crate) async fn stop<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.player().stop()
}

#[tauri::command]
pub(crate) async fn next<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.player().next()
}

#[tauri::command]
pub(crate) async fn previous<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.player().previous()
}

#[tauri::command]
pub(crate) async fn seek<R: Runtime>(app: AppHandle<R>, position_ms: i64) -> Result<()> {
    app.player().seek(position_ms)
}

#[tauri::command]
pub(crate) async fn skip_to<R: Runtime>(app: AppHandle<R>, index: i32) -> Result<()> {
    app.player().skip_to(index)
}

#[tauri::command]
pub(crate) async fn set_shuffle<R: Runtime>(app: AppHandle<R>, enabled: bool) -> Result<()> {
    app.player().set_shuffle(enabled)
}

#[tauri::command]
pub(crate) async fn set_repeat<R: Runtime>(app: AppHandle<R>, mode: RepeatMode) -> Result<()> {
    app.player().set_repeat(mode)
}

#[tauri::command]
pub(crate) async fn set_volume<R: Runtime>(app: AppHandle<R>, volume: f32) -> Result<()> {
    app.player().set_volume(volume)
}

#[tauri::command]
pub(crate) async fn set_speed<R: Runtime>(app: AppHandle<R>, speed: f32) -> Result<()> {
    app.player().set_speed(speed)
}

#[tauri::command]
pub(crate) async fn set_crossfade<R: Runtime>(app: AppHandle<R>, duration_ms: i64) -> Result<()> {
    app.player().set_crossfade(duration_ms)
}

#[tauri::command]
pub(crate) async fn add_next<R: Runtime>(app: AppHandle<R>, items: Vec<QueueItem>) -> Result<()> {
    app.player().add_next(items)
}

#[tauri::command]
pub(crate) async fn add_to_queue<R: Runtime>(
    app: AppHandle<R>,
    items: Vec<QueueItem>,
) -> Result<()> {
    app.player().add_to_queue(items)
}

#[tauri::command]
pub(crate) async fn remove_queue_item<R: Runtime>(app: AppHandle<R>, index: i32) -> Result<()> {
    app.player().remove_queue_item(index)
}

#[tauri::command]
pub(crate) async fn move_queue_item<R: Runtime>(
    app: AppHandle<R>,
    from: i32,
    to: i32,
) -> Result<()> {
    app.player().move_queue_item(from, to)
}

#[tauri::command]
pub(crate) async fn clear_queue<R: Runtime>(app: AppHandle<R>) -> Result<()> {
    app.player().clear_queue()
}

#[tauri::command]
pub(crate) async fn scan_media_store<R: Runtime>(
    app: AppHandle<R>,
    since: Option<i64>,
) -> Result<ScanBatch> {
    app.player().scan_media_store(since)
}

#[tauri::command]
pub(crate) async fn scan_tree<R: Runtime>(
    app: AppHandle<R>,
    tree_uri: String,
    since: Option<i64>,
) -> Result<ScanBatch> {
    app.player().scan_tree(tree_uri, since)
}

#[tauri::command]
pub(crate) async fn pick_folder<R: Runtime>(app: AppHandle<R>) -> Result<Option<String>> {
    app.player().pick_folder()
}

#[tauri::command]
pub(crate) async fn persisted_roots<R: Runtime>(app: AppHandle<R>) -> Result<Vec<String>> {
    app.player().persisted_roots()
}

#[tauri::command]
pub(crate) async fn release_root<R: Runtime>(app: AppHandle<R>, tree_uri: String) -> Result<()> {
    app.player().release_root(tree_uri)
}

#[tauri::command]
pub(crate) async fn extract_artwork<R: Runtime>(
    app: AppHandle<R>,
    uri: String,
) -> Result<Option<String>> {
    app.player().extract_artwork(uri)
}

#[tauri::command]
pub(crate) async fn delete_track_file<R: Runtime>(
    app: AppHandle<R>,
    uri: String,
) -> Result<DeleteFileResponse> {
    blocking_plugin_call(move || app.player().delete_track_file(uri)).await
}

#[tauri::command]
pub(crate) async fn track_file_exists<R: Runtime>(
    app: AppHandle<R>,
    uri: String,
) -> Result<TrackFileExistsResponse> {
    blocking_plugin_call(move || app.player().track_file_exists(uri)).await
}

#[cfg(test)]
mod tests {
    use super::blocking_plugin_call;

    #[test]
    fn direct_plugin_calls_run_off_the_calling_thread() {
        let caller = std::thread::current().id();
        let worker = tauri::async_runtime::block_on(blocking_plugin_call(move || {
            Ok(std::thread::current().id())
        }))
        .expect("blocking call succeeds");

        assert_ne!(worker, caller);
    }
}
