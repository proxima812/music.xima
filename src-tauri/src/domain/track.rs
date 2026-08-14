use serde::{Deserialize, Serialize};

use crate::domain::sort::normalize_sort_key;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: i64,
    /// content:// URI (MediaStore or SAF). Never an absolute path.
    pub uri: String,
    pub title: String,
    pub artist_id: Option<i64>,
    pub artist_name: Option<String>,
    pub album_id: Option<i64>,
    pub album_title: Option<String>,
    pub duration_ms: i64,
    pub track_number: Option<i32>,
    pub disc_number: Option<i32>,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub bitrate: Option<i32>,
    pub sample_rate: Option<i32>,
    pub size: i64,
    pub format: Option<String>,
    /// Artwork key in the app cache; resolved by the `artwork_uri` command.
    pub cover_key: Option<String>,
    /// Parent folder (Folders mode), display path from SAF/MediaStore.
    pub folder: Option<String>,
    pub date_added: i64,
    pub last_modified: i64,
    pub is_favorite: bool,
    pub play_count: i64,
    pub last_played_at: Option<i64>,
}

impl Track {
    /// Key for the `tracks.sort_title` column.
    pub fn sort_title(&self) -> String {
        normalize_sort_key(&self.title)
    }
}

#[cfg(test)]
mod tests {
    use super::Track;

    fn track() -> Track {
        Track {
            id: 1,
            uri: "content://media/external/audio/media/42".to_owned(),
            title: "The Bends".to_owned(),
            artist_id: Some(7),
            artist_name: Some("Radiohead".to_owned()),
            album_id: Some(3),
            album_title: Some("The Bends".to_owned()),
            duration_ms: 240_000,
            track_number: Some(4),
            disc_number: Some(1),
            year: Some(1995),
            genre: Some("Alternative".to_owned()),
            bitrate: Some(320_000),
            sample_rate: Some(44_100),
            size: 9_600_000,
            format: Some("MP3".to_owned()),
            cover_key: Some("cover-3".to_owned()),
            folder: Some("Music/Radiohead".to_owned()),
            date_added: 1_700_000_000_000,
            last_modified: 1_700_000_000_000,
            is_favorite: true,
            play_count: 12,
            last_played_at: Some(1_700_500_000_000),
        }
    }

    #[test]
    fn sort_title_drops_article() {
        assert_eq!(track().sort_title(), "bends");
    }

    #[test]
    fn serializes_in_camel_case() {
        let value = serde_json::to_value(track()).expect("track serializes");
        let object = value.as_object().expect("track is a json object");
        assert!(object.contains_key("artistId"));
        assert!(object.contains_key("albumTitle"));
        assert!(object.contains_key("durationMs"));
        assert!(object.contains_key("isFavorite"));
        assert!(object.contains_key("lastPlayedAt"));
        assert_eq!(
            object.get("coverKey").and_then(|v| v.as_str()),
            Some("cover-3")
        );
    }

    #[test]
    fn round_trips_through_json() {
        let json = serde_json::to_string(&track()).expect("serializes");
        let back: Track = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, track());
    }
}
