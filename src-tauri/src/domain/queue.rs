//! Playback state mirrored from the native player (CONTRACTS §1.5).
//!
//! The native player owns this state. Rust never derives or caches it, it only
//! forwards what the plugin reports.

use serde::{Deserialize, Serialize};

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

impl PlaybackStatus {
    /// The session is attached to a track, whether or not audio is flowing.
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Buffering | Self::Playing | Self::Paused)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "IDLE",
            Self::Buffering => "BUFFERING",
            Self::Playing => "PLAYING",
            Self::Paused => "PAUSED",
            Self::Ended => "ENDED",
        }
    }
}

impl RepeatMode {
    /// Stable token, identical to the JSON representation. Used to persist the
    /// mode in settings.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "OFF",
            Self::All => "ALL",
            Self::One => "ONE",
        }
    }

    pub fn from_token(token: &str) -> Option<Self> {
        [Self::Off, Self::All, Self::One]
            .into_iter()
            .find(|mode| mode.as_str() == token)
    }
}

impl PlaybackState {
    /// The state a player reports when nothing was ever loaded; also what the
    /// desktop stub returns.
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

    pub const fn is_playing(&self) -> bool {
        matches!(self.status, PlaybackStatus::Playing)
    }
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self::idle()
    }
}

#[cfg(test)]
mod tests {
    use super::{PlaybackState, PlaybackStatus, RepeatMode};

    #[test]
    fn idle_state_is_neutral() {
        let state = PlaybackState::idle();
        assert_eq!(state, PlaybackState::default());
        assert_eq!(state.status, PlaybackStatus::Idle);
        assert!(!state.is_playing());
        assert!(state.track_id.is_none());
        assert!((state.volume - 1.0).abs() < f32::EPSILON);
        assert!((state.speed - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn status_activity() {
        assert!(PlaybackStatus::Playing.is_active());
        assert!(PlaybackStatus::Paused.is_active());
        assert!(PlaybackStatus::Buffering.is_active());
        assert!(!PlaybackStatus::Idle.is_active());
        assert!(!PlaybackStatus::Ended.is_active());
    }

    #[test]
    fn enum_tokens_match_json() {
        for status in [
            PlaybackStatus::Idle,
            PlaybackStatus::Buffering,
            PlaybackStatus::Playing,
            PlaybackStatus::Paused,
            PlaybackStatus::Ended,
        ] {
            let json = serde_json::to_string(&status).expect("status serializes");
            assert_eq!(json, format!("\"{}\"", status.as_str()));
        }

        for mode in [RepeatMode::Off, RepeatMode::All, RepeatMode::One] {
            let json = serde_json::to_string(&mode).expect("mode serializes");
            assert_eq!(json, format!("\"{}\"", mode.as_str()));
            assert_eq!(RepeatMode::from_token(mode.as_str()), Some(mode));
        }
        assert_eq!(RepeatMode::from_token("off"), None);
    }

    #[test]
    fn state_serializes_in_camel_case() {
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
        assert_eq!(value.get("trackId").and_then(|v| v.as_i64()), Some(12));
        assert_eq!(
            value.get("positionMs").and_then(|v| v.as_i64()),
            Some(1_000)
        );
        assert_eq!(value.get("queueIndex").and_then(|v| v.as_i64()), Some(2));
        assert_eq!(value.get("queueLength").and_then(|v| v.as_i64()), Some(30));
        assert_eq!(value.get("repeat").and_then(|v| v.as_str()), Some("ALL"));

        let back: PlaybackState = serde_json::from_value(value).expect("state deserializes");
        assert_eq!(back, state);
    }
}
