use serde::{Deserialize, Serialize};

use crate::domain::sort::normalize_sort_key;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: i64,
    pub title: String,
    pub artist_id: Option<i64>,
    pub artist_name: Option<String>,
    pub year: Option<i32>,
    pub cover_key: Option<String>,
    pub track_count: i64,
    pub duration_ms: i64,
}

impl Album {
    /// Key for the `albums.sort_title` column.
    pub fn sort_title(&self) -> String {
        normalize_sort_key(&self.title)
    }
}

#[cfg(test)]
mod tests {
    use super::Album;

    fn album() -> Album {
        Album {
            id: 3,
            title: "A Moon Shaped Pool".to_owned(),
            artist_id: Some(7),
            artist_name: Some("Radiohead".to_owned()),
            year: Some(2016),
            cover_key: None,
            track_count: 11,
            duration_ms: 3_180_000,
        }
    }

    #[test]
    fn sort_title_drops_article() {
        assert_eq!(album().sort_title(), "moon shaped pool");
    }

    #[test]
    fn serializes_in_camel_case_with_null_cover() {
        let value = serde_json::to_value(album()).expect("album serializes");
        assert_eq!(value.get("trackCount").and_then(|v| v.as_i64()), Some(11));
        assert_eq!(
            value.get("artistName").and_then(|v| v.as_str()),
            Some("Radiohead")
        );
        assert!(value.get("coverKey").map(|v| v.is_null()).unwrap_or(false));
    }
}
