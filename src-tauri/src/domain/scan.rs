//! What the native scanner hands over (CONTRACTS §3) plus the scan progress /
//! result types the library commands return (CONTRACTS §5).

use serde::{Deserialize, Serialize};

use crate::domain::sort::normalize_sort_key;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScanMode {
    #[default]
    Full,
    Incremental,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub added: i64,
    pub updated: i64,
    pub removed: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub running: bool,
    pub scanned: i64,
    pub total: i64,
    pub phase: String,
}

impl ScanMode {
    /// Incremental scans only look at items newer than the last known
    /// `last_modified`; a full scan re-reads everything.
    pub const fn is_incremental(self) -> bool {
        matches!(self, Self::Incremental)
    }

    /// `since` cursor handed to the native scanner.
    pub const fn since(self, last_scan_ms: Option<i64>) -> Option<i64> {
        match self {
            Self::Full => None,
            Self::Incremental => last_scan_ms,
        }
    }
}

impl ScanStatus {
    pub fn idle() -> Self {
        Self {
            running: false,
            scanned: 0,
            total: 0,
            phase: PHASE_IDLE.to_owned(),
        }
    }

    pub fn in_progress(phase: impl Into<String>, scanned: i64, total: i64) -> Self {
        Self {
            running: true,
            scanned,
            total,
            phase: phase.into(),
        }
    }

    /// 0.0..=1.0, or `None` while the total is still unknown.
    pub fn progress(&self) -> Option<f32> {
        if self.total <= 0 {
            return None;
        }
        let ratio = self.scanned as f32 / self.total as f32;
        Some(ratio.clamp(0.0, 1.0))
    }
}

pub const PHASE_IDLE: &str = "idle";
pub const PHASE_ENUMERATING: &str = "enumerating";
pub const PHASE_READING: &str = "reading";
pub const PHASE_WRITING: &str = "writing";
pub const PHASE_CLEANUP: &str = "cleanup";
pub const PHASE_DONE: &str = "done";

impl ScannedTrack {
    /// Key for the `tracks.sort_title` column.
    pub fn sort_title(&self) -> String {
        normalize_sort_key(&self.title)
    }

    /// Artist an album is filed under: the tag when present, the track artist
    /// otherwise.
    pub fn effective_album_artist(&self) -> Option<&str> {
        self.album_artist
            .as_deref()
            .or(self.artist.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    /// Human-readable container label for `tracks.format`, derived from the
    /// MIME type reported by MediaStore / SAF.
    pub fn format_label(&self) -> Option<String> {
        self.mime_type.as_deref().and_then(format_from_mime)
    }
}

/// "audio/mpeg" -> "MP3", "audio/x-flac" -> "FLAC", unknown subtypes are
/// uppercased as-is. `None` when the MIME type carries no subtype.
pub fn format_from_mime(mime: &str) -> Option<String> {
    let lowered = mime.trim().to_ascii_lowercase();
    let without_params = lowered.split(';').next().unwrap_or_default().trim();
    let subtype = match without_params.split_once('/') {
        Some((_, subtype)) => subtype,
        None => without_params,
    };
    let subtype = subtype.trim_start_matches("x-").trim_start_matches("vnd.");
    if subtype.is_empty() {
        return None;
    }

    let label = match subtype {
        "mpeg" | "mpeg3" | "mp3" => "MP3",
        "mp4" | "m4a" | "mp4a-latm" => "M4A",
        "aac" | "aacp" => "AAC",
        "flac" => "FLAC",
        "ogg" | "vorbis" => "OGG",
        "opus" => "OPUS",
        "wav" | "wave" | "pn-wav" => "WAV",
        "ms-wma" | "wma" => "WMA",
        "aiff" | "aif" => "AIFF",
        "matroska" => "MKA",
        "3gpp" | "3gpp2" => "3GP",
        "midi" | "mid" => "MIDI",
        "ape" | "monkeys-audio" => "APE",
        other => return Some(other.to_uppercase()),
    };
    Some(label.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{format_from_mime, ScanMode, ScanStatus, ScannedTrack, PHASE_READING};

    fn scanned() -> ScannedTrack {
        ScannedTrack {
            uri: "content://com.android.externalstorage.documents/tree/x/1".to_owned(),
            title: "The Bends".to_owned(),
            artist: Some("Radiohead".to_owned()),
            album: Some("The Bends".to_owned()),
            album_artist: None,
            duration_ms: 240_000,
            track_number: Some(4),
            disc_number: Some(1),
            year: Some(1995),
            genre: Some("Alternative".to_owned()),
            bitrate: Some(320_000),
            sample_rate: Some(44_100),
            size: 9_600_000,
            mime_type: Some("audio/mpeg".to_owned()),
            folder: Some("Music/Radiohead".to_owned()),
            date_added: 1_700_000_000_000,
            last_modified: 1_700_000_000_000,
            cover_key: None,
        }
    }

    #[test]
    fn serializes_in_camel_case() {
        let value = serde_json::to_value(scanned()).expect("scanned track serializes");
        let object = value.as_object().expect("scanned track is a json object");
        assert!(object.contains_key("albumArtist"));
        assert!(object.contains_key("mimeType"));
        assert!(object.contains_key("lastModified"));
        assert!(object.contains_key("sampleRate"));
    }

    #[test]
    fn sort_title_drops_article() {
        assert_eq!(scanned().sort_title(), "bends");
    }

    #[test]
    fn album_artist_falls_back_to_track_artist() {
        let mut track = scanned();
        assert_eq!(track.effective_album_artist(), Some("Radiohead"));

        track.album_artist = Some("  Thom Yorke ".to_owned());
        assert_eq!(track.effective_album_artist(), Some("Thom Yorke"));

        track.album_artist = Some("  ".to_owned());
        assert_eq!(track.effective_album_artist(), None);

        track.album_artist = None;
        track.artist = None;
        assert_eq!(track.effective_album_artist(), None);
    }

    #[test]
    fn maps_known_mime_types() {
        assert_eq!(format_from_mime("audio/mpeg").as_deref(), Some("MP3"));
        assert_eq!(format_from_mime("audio/x-flac").as_deref(), Some("FLAC"));
        assert_eq!(format_from_mime("AUDIO/FLAC").as_deref(), Some("FLAC"));
        assert_eq!(format_from_mime("audio/mp4").as_deref(), Some("M4A"));
        assert_eq!(
            format_from_mime("audio/ogg; codecs=opus").as_deref(),
            Some("OGG")
        );
        assert_eq!(format_from_mime("audio/opus").as_deref(), Some("OPUS"));
        assert_eq!(format_from_mime("audio/x-ms-wma").as_deref(), Some("WMA"));
        assert_eq!(scanned().format_label().as_deref(), Some("MP3"));
    }

    #[test]
    fn falls_back_to_uppercased_subtype() {
        assert_eq!(
            format_from_mime("audio/weirdcodec").as_deref(),
            Some("WEIRDCODEC")
        );
        assert_eq!(format_from_mime("dsf").as_deref(), Some("DSF"));
        assert_eq!(format_from_mime(""), None);
        assert_eq!(format_from_mime("audio/"), None);
    }

    #[test]
    fn scan_mode_controls_cursor() {
        assert!(ScanMode::Incremental.is_incremental());
        assert!(!ScanMode::default().is_incremental());
        assert_eq!(ScanMode::Full.since(Some(10)), None);
        assert_eq!(ScanMode::Incremental.since(Some(10)), Some(10));
        assert_eq!(ScanMode::Incremental.since(None), None);
    }

    #[test]
    fn status_progress() {
        assert_eq!(ScanStatus::idle().progress(), None);
        assert!(!ScanStatus::idle().running);

        let status = ScanStatus::in_progress(PHASE_READING, 5, 10);
        assert_eq!(status.progress(), Some(0.5));
        assert_eq!(status.phase, PHASE_READING);
        assert!(status.running);

        let overshoot = ScanStatus::in_progress(PHASE_READING, 20, 10);
        assert_eq!(overshoot.progress(), Some(1.0));
    }
}
