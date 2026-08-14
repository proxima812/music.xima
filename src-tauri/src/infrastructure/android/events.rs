//! Native events re-emitted as Tauri events (CONTRACTS §6, §7.2).
//!
//! The Kotlin plugin triggers `state` / `trackChanged` / `queueChanged` /
//! `completed` / `error` / `scanProgress`; this module is the single place that
//! turns them into the `player:*` / `library:*` events the frontend listens to.
//! History is written here too: `completed` reaches the database before the
//! frontend hears about it, so the UI never renders a play count the backend
//! has not stored yet.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};
use tauri_plugin_player::{PlayerEvent, PlayerExt};

use crate::application::{HistoryService, PlayerService, ScanService};
use crate::domain::ScanStatus;
use crate::error::CoreResult;
use crate::infrastructure::android::player_adapter::{playback_state, AndroidPlayerAdapter};

/// Event names from CONTRACTS §6. Mirrored in `src/shared/ipc/events.ts`.
pub const PLAYER_STATE: &str = "player:state";
pub const PLAYER_TRACK_CHANGED: &str = "player:track-changed";
pub const PLAYER_QUEUE_CHANGED: &str = "player:queue-changed";
pub const PLAYER_COMPLETED: &str = "player:completed";
pub const PLAYER_ERROR: &str = "player:error";
pub const LIBRARY_SCAN_PROGRESS: &str = "library:scan-progress";
pub const LIBRARY_CHANGED: &str = "library:changed";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackChangedPayload {
    pub track_id: Option<i64>,
    pub index: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueChangedPayload {
    pub track_ids: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletedPayload {
    pub track_id: i64,
    pub duration_played_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerErrorPayload {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryChangedPayload {
    pub reason: String,
}

/// Everything a native event has to reach. Built once in the composition root
/// and moved into the subscription.
pub struct EventSinks<R: Runtime> {
    /// Queue mirror behind `PlayerPort::queue_ids`.
    pub queue: Arc<AndroidPlayerAdapter<R>>,
    pub player: Arc<PlayerService>,
    pub history: Arc<HistoryService>,
    pub scan: Arc<ScanService>,
}

/// Subscribes to the native player for the lifetime of the app. On targets
/// without Media3 the plugin stub never fires, so this ends up a no-op.
pub fn subscribe<R: Runtime>(app: &AppHandle<R>, sinks: EventSinks<R>) -> CoreResult<()> {
    let handle = app.clone();
    app.player().on_event(move |event| {
        dispatch(&handle, &sinks, event);
    })?;
    Ok(())
}

/// Tells the frontend to re-read the library. `reason` is free-form and used
/// for coarse invalidation ("scan", "root-removed", ...).
pub fn emit_library_changed<R: Runtime>(app: &AppHandle<R>, reason: impl Into<String>) {
    emit(
        app,
        LIBRARY_CHANGED,
        LibraryChangedPayload {
            reason: reason.into(),
        },
    );
}

/// Progress of a scan driven by `ScanService`. Wired as its
/// `ProgressCallback`, which is the only producer of this event.
pub fn emit_scan_progress<R: Runtime>(app: &AppHandle<R>, status: ScanStatus) {
    emit(app, LIBRARY_SCAN_PROGRESS, status);
}

fn dispatch<R: Runtime>(app: &AppHandle<R>, sinks: &EventSinks<R>, event: PlayerEvent) {
    match event {
        PlayerEvent::State(state) => {
            emit(app, PLAYER_STATE, playback_state(state));
        }
        PlayerEvent::TrackChanged { track_id, index } => {
            emit(
                app,
                PLAYER_TRACK_CHANGED,
                TrackChangedPayload { track_id, index },
            );
        }
        PlayerEvent::QueueChanged { track_ids } => {
            // The native side just told us the truth: both mirrors follow it.
            sinks.queue.sync_queue(track_ids.clone());
            sinks.player.sync_queue(track_ids.clone());
            emit(app, PLAYER_QUEUE_CHANGED, QueueChangedPayload { track_ids });
        }
        PlayerEvent::Completed {
            track_id,
            duration_played_ms,
        } => {
            record_completion(
                app.clone(),
                sinks.history.clone(),
                track_id,
                duration_played_ms,
            );
        }
        PlayerEvent::PlaybackError { code, message } => {
            log::warn!("player error {code}: {message}");
            emit(app, PLAYER_ERROR, PlayerErrorPayload { code, message });
        }
        PlayerEvent::ScanProgress {
            scanned,
            total,
            phase,
        } => {
            // Folded into the service's own status, which re-publishes it
            // through the progress callback — one event source, one shape.
            sinks.scan.report_native_progress(scanned, total, &phase);
        }
    }
}

/// History is written before the event goes out; a failed write is logged and
/// the frontend is still told the track finished.
fn record_completion<R: Runtime>(
    app: AppHandle<R>,
    history: Arc<HistoryService>,
    track_id: i64,
    duration_played_ms: i64,
) {
    tauri::async_runtime::spawn(async move {
        if let Err(error) = history
            .on_track_completed(track_id, duration_played_ms)
            .await
        {
            log::error!("history: track {track_id} not recorded: {error}");
        }
        emit(
            &app,
            PLAYER_COMPLETED,
            CompletedPayload {
                track_id,
                duration_played_ms,
            },
        );
    });
}

/// Emitting only fails when the window is already gone, which is not worth
/// propagating out of an event handler.
fn emit<R: Runtime, P: Serialize + Clone>(app: &AppHandle<R>, event: &str, payload: P) {
    if let Err(error) = app.emit(event, payload) {
        log::warn!("event {event} not delivered: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CompletedPayload, LibraryChangedPayload, PlayerErrorPayload, QueueChangedPayload,
        TrackChangedPayload, LIBRARY_CHANGED, LIBRARY_SCAN_PROGRESS, PLAYER_COMPLETED,
        PLAYER_ERROR, PLAYER_QUEUE_CHANGED, PLAYER_STATE, PLAYER_TRACK_CHANGED,
    };
    use serde_json::json;

    #[test]
    fn event_names_match_the_contract() {
        assert_eq!(PLAYER_STATE, "player:state");
        assert_eq!(PLAYER_TRACK_CHANGED, "player:track-changed");
        assert_eq!(PLAYER_QUEUE_CHANGED, "player:queue-changed");
        assert_eq!(PLAYER_COMPLETED, "player:completed");
        assert_eq!(PLAYER_ERROR, "player:error");
        assert_eq!(LIBRARY_SCAN_PROGRESS, "library:scan-progress");
        assert_eq!(LIBRARY_CHANGED, "library:changed");
    }

    #[test]
    fn payloads_are_camel_case() {
        assert_eq!(
            serde_json::to_value(TrackChangedPayload {
                track_id: None,
                index: 3,
            })
            .expect("serializes"),
            json!({ "trackId": null, "index": 3 })
        );
        assert_eq!(
            serde_json::to_value(QueueChangedPayload {
                track_ids: vec![1, 2],
            })
            .expect("serializes"),
            json!({ "trackIds": [1, 2] })
        );
        assert_eq!(
            serde_json::to_value(CompletedPayload {
                track_id: 7,
                duration_played_ms: 180_000,
            })
            .expect("serializes"),
            json!({ "trackId": 7, "durationPlayedMs": 180_000 })
        );
        assert_eq!(
            serde_json::to_value(PlayerErrorPayload {
                code: "IO".to_owned(),
                message: "boom".to_owned(),
            })
            .expect("serializes"),
            json!({ "code": "IO", "message": "boom" })
        );
        assert_eq!(
            serde_json::to_value(LibraryChangedPayload {
                reason: "scan".to_owned(),
            })
            .expect("serializes"),
            json!({ "reason": "scan" })
        );
    }
}
