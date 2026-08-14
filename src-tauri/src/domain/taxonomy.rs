//! Browse-only groupings of the library: genres and folders (CONTRACTS §1.2).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub name: String,
    pub track_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    /// Display path, e.g. "Music/MyMusic/Rock".
    pub path: String,
    pub name: String,
    pub track_count: i64,
}

/// Separator used in the display paths stored in `tracks.folder`.
pub const FOLDER_SEPARATOR: char = '/';

/// Last segment of a display path; the whole path when there is no separator.
pub fn folder_name(path: &str) -> &str {
    let trimmed = path.trim_end_matches(FOLDER_SEPARATOR);
    match trimmed.rsplit_once(FOLDER_SEPARATOR) {
        Some((_, name)) => name,
        None => trimmed,
    }
}

/// Parent of a display path, `None` for a top-level folder.
pub fn folder_parent(path: &str) -> Option<&str> {
    let trimmed = path.trim_end_matches(FOLDER_SEPARATOR);
    match trimmed.rsplit_once(FOLDER_SEPARATOR) {
        Some((parent, _)) if !parent.is_empty() => Some(parent),
        _ => None,
    }
}

impl Folder {
    /// Builds a folder entry from a display path, deriving the visible name.
    pub fn from_path(path: impl Into<String>, track_count: i64) -> Self {
        let path = path.into();
        let name = folder_name(&path).to_owned();
        Self {
            path,
            name,
            track_count,
        }
    }

    pub fn parent(&self) -> Option<&str> {
        folder_parent(&self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::{folder_name, folder_parent, Folder, Genre};

    #[test]
    fn genre_serializes_in_camel_case() {
        let value = serde_json::to_value(Genre {
            name: "Trip-Hop".to_owned(),
            track_count: 34,
        })
        .expect("genre serializes");
        assert_eq!(value.get("trackCount").and_then(|v| v.as_i64()), Some(34));
    }

    #[test]
    fn derives_name_from_path() {
        assert_eq!(folder_name("Music/MyMusic/Rock"), "Rock");
        assert_eq!(folder_name("Music"), "Music");
        assert_eq!(folder_name("Music/MyMusic/"), "MyMusic");
        assert_eq!(folder_name(""), "");
    }

    #[test]
    fn derives_parent_from_path() {
        assert_eq!(folder_parent("Music/MyMusic/Rock"), Some("Music/MyMusic"));
        assert_eq!(folder_parent("Music"), None);
        assert_eq!(folder_parent("/Music"), None);
        assert_eq!(folder_parent("Music/MyMusic/"), Some("Music"));
    }

    #[test]
    fn from_path_fills_name() {
        let folder = Folder::from_path("Music/MyMusic/Rock", 12);
        assert_eq!(folder.name, "Rock");
        assert_eq!(folder.parent(), Some("Music/MyMusic"));
        assert_eq!(folder.track_count, 12);
    }
}
