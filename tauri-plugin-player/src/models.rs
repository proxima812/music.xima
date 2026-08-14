//! Wire types shared with the Kotlin side (CONTRACTS §7.1).
//!
//! The plugin deliberately does not depend on the application crate: it must be
//! buildable on its own, so these DTOs are duplicated here. Field names, casing
//! and nullability are identical to `src-tauri/src/domain`.

use serde::{Deserialize, Serialize};

/// Kotlin `trigger()` event names (CONTRACTS §7.2).
pub const EVENT_STATE: &str = "state";
pub const EVENT_TRACK_CHANGED: &str = "trackChanged";
pub const EVENT_QUEUE_CHANGED: &str = "queueChanged";
pub const EVENT_COMPLETED: &str = "completed";
pub const EVENT_ERROR: &str = "error";
pub const EVENT_SCAN_PROGRESS: &str = "scanProgress";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlaybackStatus {
    #[default]
    Idle,
    Buffering,
    Playing,
    Paused,
    Ended,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepeatMode {
    #[default]
    Off,
    All,
    One,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackState {
    pub status: PlaybackStatus,
    pub track_id: Option<i64>,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub queue_index: Option<i32>,
    pub queue_length: i32,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub volume: f32,
    pub speed: f32,
}

impl PlaybackState {
    /// Nothing loaded: what a fresh session reports and what the desktop stub
    /// always returns.
    pub fn idle() -> Self {
        Self {
            status: PlaybackStatus::Idle,
            track_id: None,
            position_ms: 0,
            duration_ms: 0,
            queue_index: None,
            queue_length: 0,
            shuffle: false,
            repeat: RepeatMode::Off,
            volume: 1.0,
            speed: 1.0,
        }
    }
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self::idle()
    }
}

/// Everything the native player needs to play a track and draw the
/// notification. Ids stay in sync with the Rust library so events can be
/// mapped back to database rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub track_id: i64,
    pub uri: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: i64,
    /// `file://` or `content://` pointing at the artwork cache.
    pub artwork_uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetQueueRequest {
    pub items: Vec<QueueItem>,
    pub start_index: i32,
    pub autoplay: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedTrack {
    pub uri: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub duration_ms: i64,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub size: i64,
    pub mime_type: Option<String>,
    pub folder: Option<String>,
    pub date_added: i64,
    pub last_modified: i64,
    pub cover_key: Option<String>,
}

/// One page of a scan. `complete = false` means the caller should ask again
/// passing `next_cursor` back to the native side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanBatch {
    pub tracks: Vec<ScannedTrack>,
    pub complete: bool,
    pub next_cursor: Option<String>,
}

impl ScanBatch {
    /// A finished scan that found nothing — the desktop stub's answer.
    pub fn empty() -> Self {
        Self {
            tracks: Vec::new(),
            complete: true,
            next_cursor: None,
        }
    }
}

/// Decoded `trigger()` payloads (CONTRACTS §7.2). The core crate re-emits these
/// as the Tauri events of §6.
#[derive(Debug, Clone, PartialEq)]
pub enum PlayerEvent {
    State(PlaybackState),
    TrackChanged {
        track_id: Option<i64>,
        index: i32,
    },
    QueueChanged {
        track_ids: Vec<i64>,
    },
    Completed {
        track_id: i64,
        duration_played_ms: i64,
    },
    PlaybackError {
        code: String,
        message: String,
    },
    ScanProgress {
        scanned: i64,
        total: i64,
        phase: String,
    },
}

impl PlayerEvent {
    /// The Kotlin `trigger()` name this variant was decoded from.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::State(_) => EVENT_STATE,
            Self::TrackChanged { .. } => EVENT_TRACK_CHANGED,
            Self::QueueChanged { .. } => EVENT_QUEUE_CHANGED,
            Self::Completed { .. } => EVENT_COMPLETED,
            Self::PlaybackError { .. } => EVENT_ERROR,
            Self::ScanProgress { .. } => EVENT_SCAN_PROGRESS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaybackState, PlaybackStatus, PlayerEvent, QueueItem, RepeatMode, ScanBatch};

    #[test]
    fn idle_state_is_neutral() {
        let state = PlaybackState::idle();
        assert_eq!(state, PlaybackState::default());
        assert_eq!(state.status, PlaybackStatus::Idle);
        assert_eq!(state.repeat, RepeatMode::Off);
        assert!(state.track_id.is_none());
        assert!((state.volume - 1.0).abs() < f32::EPSILON);
        assert!((state.speed - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn enums_serialize_as_screaming_snake_case() {
        assert_eq!(
            serde_json::to_string(&PlaybackStatus::Buffering).expect("status serializes"),
            "\"BUFFERING\""
        );
        assert_eq!(
            serde_json::to_string(&RepeatMode::One).expect("mode serializes"),
            "\"ONE\""
        );
    }

    #[test]
    fn queue_item_serializes_in_camel_case() {
        let item = QueueItem {
            track_id: 7,
            uri: "content://media/external/audio/media/7".into(),
            title: "Song".into(),
            artist: None,
            album: Some("Album".into()),
            duration_ms: 180_000,
            artwork_uri: None,
        };
        let value = serde_json::to_value(&item).expect("item serializes");
        assert_eq!(value.get("trackId").and_then(|v| v.as_i64()), Some(7));
        assert_eq!(
            value.get("durationMs").and_then(|v| v.as_i64()),
            Some(180_000)
        );
        assert!(value.get("artworkUri").is_some_and(|v| v.is_null()));

        let back: QueueItem = serde_json::from_value(value).expect("item deserializes");
        assert_eq!(back, item);
    }

    #[test]
    fn state_round_trips_through_json() {
        let state = PlaybackState {
            status: PlaybackStatus::Playing,
            track_id: Some(12),
            position_ms: 1_000,
            duration_ms: 240_000,
            queue_index: Some(2),
            queue_length: 30,
            shuffle: true,
            repeat: RepeatMode::All,
            volume: 0.8,
            speed: 1.0,
        };
        let value = serde_json::to_value(&state).expect("state serializes");
        assert_eq!(value.get("queueLength").and_then(|v| v.as_i64()), Some(30));
        assert_eq!(value.get("repeat").and_then(|v| v.as_str()), Some("ALL"));

        let back: PlaybackState = serde_json::from_value(value).expect("state deserializes");
        assert_eq!(back, state);
    }

    #[test]
    fn empty_batch_is_complete() {
        let batch = ScanBatch::empty();
        assert!(batch.complete);
        assert!(batch.tracks.is_empty());
        assert!(batch.next_cursor.is_none());
    }

    #[test]
    fn event_names_match_the_kotlin_triggers() {
        assert_eq!(PlayerEvent::State(PlaybackState::idle()).name(), "state");
        assert_eq!(
            PlayerEvent::TrackChanged {
                track_id: None,
                index: 0
            }
            .name(),
            "trackChanged"
        );
        assert_eq!(
            PlayerEvent::QueueChanged {
                track_ids: Vec::new()
            }
            .name(),
            "queueChanged"
        );
        assert_eq!(
            PlayerEvent::Completed {
                track_id: 1,
                duration_played_ms: 0
            }
            .name(),
            "completed"
        );
        assert_eq!(
            PlayerEvent::PlaybackError {
                code: "IO".into(),
                message: String::new()
            }
            .name(),
            "error"
        );
        assert_eq!(
            PlayerEvent::ScanProgress {
                scanned: 0,
                total: 0,
                phase: "IDLE".into()
            }
            .name(),
            "scanProgress"
        );
    }
}
