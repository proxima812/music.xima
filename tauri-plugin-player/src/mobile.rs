//! Android bridge. Every method is a single `run_mobile_plugin` call into
//! `PlayerPlugin.kt`; the command names are the camelCase ones from
//! CONTRACTS §7.2 and must not drift.

use std::sync::Arc;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tauri::{
    ipc::{Channel, InvokeResponseBody},
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::error::{Error, Result};
use crate::models::{
    DeleteFileResponse, PlaybackState, PlayerEvent, QueueIdsResponse, QueueItem, RepeatMode,
    ScanBatch, SetQueueRequest, TrackFileExistsResponse, EVENT_COMPLETED, EVENT_ERROR,
    EVENT_QUEUE_CHANGED, EVENT_SCAN_PROGRESS, EVENT_STATE, EVENT_TRACK_CHANGED,
};

const PLUGIN_IDENTIFIER: &str = "com.xima.music.player";
const PLUGIN_CLASS: &str = "PlayerPlugin";

/// Built into `app.tauri.plugin.Plugin`: hands a channel to the Kotlin side,
/// which then feeds it from `trigger(event, payload)`.
const REGISTER_LISTENER: &str = "registerListener";

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> Result<Player<R>> {
    let handle = api.register_android_plugin(PLUGIN_IDENTIFIER, PLUGIN_CLASS)?;
    Ok(Player(handle))
}

pub struct Player<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> Player<R> {
    pub fn get_state(&self) -> Result<PlaybackState> {
        Ok(self.0.run_mobile_plugin("getState", ())?)
    }

    pub fn get_queue_ids(&self) -> Result<QueueIdsResponse> {
        Ok(self.0.run_mobile_plugin("getQueueIds", ())?)
    }

    pub fn set_queue(&self, req: SetQueueRequest) -> Result<()> {
        Ok(self.0.run_mobile_plugin::<()>("setQueue", req)?)
    }

    pub fn play(&self) -> Result<()> {
        Ok(self.0.run_mobile_plugin::<()>("play", ())?)
    }

    pub fn pause(&self) -> Result<()> {
        Ok(self.0.run_mobile_plugin::<()>("pause", ())?)
    }

    pub fn toggle(&self) -> Result<()> {
        Ok(self.0.run_mobile_plugin::<()>("toggle", ())?)
    }

    pub fn stop(&self) -> Result<()> {
        Ok(self.0.run_mobile_plugin::<()>("stop", ())?)
    }

    pub fn next(&self) -> Result<()> {
        Ok(self.0.run_mobile_plugin::<()>("next", ())?)
    }

    pub fn previous(&self) -> Result<()> {
        Ok(self.0.run_mobile_plugin::<()>("previous", ())?)
    }

    pub fn seek(&self, position_ms: i64) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("seek", SeekArgs { position_ms })?)
    }

    pub fn skip_to(&self, index: i32) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("skipTo", IndexArgs { index })?)
    }

    pub fn set_shuffle(&self, enabled: bool) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("setShuffle", ShuffleArgs { enabled })?)
    }

    pub fn set_repeat(&self, mode: RepeatMode) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("setRepeat", RepeatArgs { mode })?)
    }

    pub fn set_volume(&self, volume: f32) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("setVolume", VolumeArgs { volume })?)
    }

    pub fn set_speed(&self, speed: f32) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("setSpeed", SpeedArgs { speed })?)
    }

    pub fn add_next(&self, items: Vec<QueueItem>) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("addNext", ItemsArgs { items })?)
    }

    pub fn add_to_queue(&self, items: Vec<QueueItem>) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("addToQueue", ItemsArgs { items })?)
    }

    pub fn remove_queue_item(&self, index: i32) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("removeQueueItem", IndexArgs { index })?)
    }

    pub fn move_queue_item(&self, from: i32, to: i32) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("moveQueueItem", MoveArgs { from, to })?)
    }

    pub fn clear_queue(&self) -> Result<()> {
        Ok(self.0.run_mobile_plugin::<()>("clearQueue", ())?)
    }

    pub fn scan_media_store(&self, since: Option<i64>) -> Result<ScanBatch> {
        Ok(self
            .0
            .run_mobile_plugin("scanMediaStore", SinceArgs { since })?)
    }

    pub fn scan_tree(&self, tree_uri: String, since: Option<i64>) -> Result<ScanBatch> {
        Ok(self
            .0
            .run_mobile_plugin("scanTree", ScanTreeArgs { tree_uri, since })?)
    }

    /// Opens `ACTION_OPEN_DOCUMENT_TREE` and returns the picked tree URI, or
    /// `None` when the user cancelled.
    pub fn pick_folder(&self) -> Result<Option<String>> {
        Ok(self.0.run_mobile_plugin("pickFolder", ())?)
    }

    pub fn persisted_roots(&self) -> Result<Vec<String>> {
        Ok(self.0.run_mobile_plugin("persistedRoots", ())?)
    }

    pub fn release_root(&self, tree_uri: String) -> Result<()> {
        Ok(self
            .0
            .run_mobile_plugin::<()>("releaseRoot", TreeUriArgs { tree_uri })?)
    }

    /// Extracts embedded artwork into the app cache and returns its `file://`
    /// URI, or `None` when the file carries no cover.
    pub fn extract_artwork(&self, uri: String) -> Result<Option<String>> {
        Ok(self
            .0
            .run_mobile_plugin("extractArtwork", UriArgs { uri })?)
    }

    pub fn delete_track_file(&self, uri: String) -> Result<DeleteFileResponse> {
        self.0
            .run_mobile_plugin("deleteTrackFile", UriArgs { uri })
            .map_err(map_track_file_error)
    }

    pub fn track_file_exists(&self, uri: String) -> Result<TrackFileExistsResponse> {
        self.0
            .run_mobile_plugin("trackFileExists", UriArgs { uri })
            .map_err(map_track_file_error)
    }

    /// Subscribes to every `trigger()` the Kotlin plugin emits (CONTRACTS §7.2).
    /// One channel per event name; malformed payloads are logged and dropped so
    /// a single bad event cannot kill the stream.
    pub fn on_event<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(PlayerEvent) + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);

        self.listen(EVENT_STATE, handler.clone(), PlayerEvent::State)?;
        self.listen(
            EVENT_TRACK_CHANGED,
            handler.clone(),
            |p: TrackChangedPayload| PlayerEvent::TrackChanged {
                track_id: p.track_id,
                index: p.index,
            },
        )?;
        self.listen(
            EVENT_QUEUE_CHANGED,
            handler.clone(),
            |p: QueueChangedPayload| PlayerEvent::QueueChanged {
                track_ids: p.track_ids,
            },
        )?;
        self.listen(EVENT_COMPLETED, handler.clone(), |p: CompletedPayload| {
            PlayerEvent::Completed {
                track_id: p.track_id,
                duration_played_ms: p.duration_played_ms,
            }
        })?;
        self.listen(EVENT_ERROR, handler.clone(), |p: ErrorPayload| {
            PlayerEvent::PlaybackError {
                code: p.code,
                message: p.message,
            }
        })?;
        self.listen(EVENT_SCAN_PROGRESS, handler, |p: ScanProgressPayload| {
            PlayerEvent::ScanProgress {
                scanned: p.scanned,
                total: p.total,
                phase: p.phase,
            }
        })?;

        Ok(())
    }

    fn listen<P, F, M>(&self, event: &'static str, handler: Arc<F>, map: M) -> Result<()>
    where
        P: DeserializeOwned,
        F: Fn(PlayerEvent) + Send + Sync + 'static,
        M: Fn(P) -> PlayerEvent + Send + Sync + 'static,
    {
        let channel = Channel::new(move |body: InvokeResponseBody| {
            match body.deserialize::<P>() {
                Ok(payload) => handler(map(payload)),
                Err(error) => {
                    log::warn!("player: dropping `{event}` event, bad payload: {error}");
                }
            }
            Ok(())
        });

        self.0.run_mobile_plugin::<()>(
            REGISTER_LISTENER,
            RegisterListenerArgs {
                event,
                handler: channel,
            },
        )?;
        Ok(())
    }
}

fn map_track_file_error(error: tauri::plugin::mobile::PluginInvokeError) -> Error {
    match &error {
        tauri::plugin::mobile::PluginInvokeError::InvokeRejected(response)
            if response.code.as_deref() == Some("UNSUPPORTED_DELETE") =>
        {
            Error::UnsupportedDelete
        }
        _ => Error::PluginInvoke(error),
    }
}

#[derive(Serialize)]
struct RegisterListenerArgs {
    event: &'static str,
    handler: Channel<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SeekArgs {
    position_ms: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexArgs {
    index: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveArgs {
    from: i32,
    to: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ShuffleArgs {
    enabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RepeatArgs {
    mode: RepeatMode,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VolumeArgs {
    volume: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SpeedArgs {
    speed: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemsArgs {
    items: Vec<QueueItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SinceArgs {
    since: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanTreeArgs {
    tree_uri: String,
    since: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TreeUriArgs {
    tree_uri: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UriArgs {
    uri: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrackChangedPayload {
    track_id: Option<i64>,
    index: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueueChangedPayload {
    track_ids: Vec<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompletedPayload {
    track_id: i64,
    duration_played_ms: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorPayload {
    code: String,
    message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanProgressPayload {
    scanned: i64,
    total: i64,
    phase: String,
}
