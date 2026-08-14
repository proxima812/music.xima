use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// Longest playlist name the UI and the database accept.
pub const MAX_PLAYLIST_NAME_LEN: usize = 120;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: i64,
    pub name: String,
    pub track_count: i64,
    pub duration_ms: i64,
    pub cover_key: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Trims a user-supplied playlist name and rejects empty / oversized ones.
pub fn sanitize_playlist_name(raw: &str) -> CoreResult<String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err(CoreError::InvalidInput("playlist name is empty".to_owned()));
    }
    if name.chars().count() > MAX_PLAYLIST_NAME_LEN {
        return Err(CoreError::InvalidInput(format!(
            "playlist name is longer than {MAX_PLAYLIST_NAME_LEN} characters"
        )));
    }
    Ok(name.to_owned())
}

/// Target index after moving an item from `from` to `to` inside a list of
/// `len` items. `None` when the move is a no-op or out of bounds.
pub fn reorder_target(len: i64, from: i64, to: i64) -> Option<i64> {
    if len <= 1 || from < 0 || to < 0 || from >= len || to >= len || from == to {
        return None;
    }
    Some(to)
}

#[cfg(test)]
mod tests {
    use super::{reorder_target, sanitize_playlist_name, Playlist, MAX_PLAYLIST_NAME_LEN};

    #[test]
    fn serializes_in_camel_case() {
        let value = serde_json::to_value(Playlist {
            id: 1,
            name: "Road trip".to_owned(),
            track_count: 21,
            duration_ms: 5_400_000,
            cover_key: None,
            created_at: 1_700_000_000_000,
            updated_at: 1_700_000_100_000,
        })
        .expect("playlist serializes");
        assert_eq!(value.get("trackCount").and_then(|v| v.as_i64()), Some(21));
        assert_eq!(
            value.get("createdAt").and_then(|v| v.as_i64()),
            Some(1_700_000_000_000)
        );
    }

    #[test]
    fn trims_names() {
        assert_eq!(
            sanitize_playlist_name("  Road trip \n").expect("valid name"),
            "Road trip"
        );
    }

    #[test]
    fn rejects_blank_names() {
        assert!(sanitize_playlist_name("   ").is_err());
        assert!(sanitize_playlist_name("").is_err());
    }

    #[test]
    fn rejects_oversized_names() {
        let long = "п".repeat(MAX_PLAYLIST_NAME_LEN + 1);
        assert!(sanitize_playlist_name(&long).is_err());
        let ok = "п".repeat(MAX_PLAYLIST_NAME_LEN);
        assert!(sanitize_playlist_name(&ok).is_ok());
    }

    #[test]
    fn reorder_rejects_noop_and_out_of_range() {
        assert_eq!(reorder_target(5, 0, 3), Some(3));
        assert_eq!(reorder_target(5, 3, 3), None);
        assert_eq!(reorder_target(5, 3, 5), None);
        assert_eq!(reorder_target(5, -1, 2), None);
        assert_eq!(reorder_target(1, 0, 0), None);
    }
}
