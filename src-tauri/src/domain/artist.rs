use serde::{Deserialize, Serialize};

use crate::domain::sort::normalize_sort_key;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: i64,
    pub name: String,
    pub album_count: i64,
    pub track_count: i64,
    pub cover_key: Option<String>,
}

impl Artist {
    /// Key for the `artists.sort_name` column, which is also the uniqueness
    /// constraint used when the scanner deduplicates artists.
    pub fn sort_name(&self) -> String {
        normalize_sort_key(&self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::Artist;

    #[test]
    fn sort_name_drops_article() {
        let artist = Artist {
            id: 7,
            name: "The Prodigy".to_owned(),
            album_count: 8,
            track_count: 96,
            cover_key: None,
        };
        assert_eq!(artist.sort_name(), "prodigy");
    }

    #[test]
    fn serializes_in_camel_case() {
        let artist = Artist {
            id: 7,
            name: "Radiohead".to_owned(),
            album_count: 9,
            track_count: 120,
            cover_key: Some("cover-artist-7".to_owned()),
        };
        let value = serde_json::to_value(artist).expect("artist serializes");
        assert_eq!(value.get("albumCount").and_then(|v| v.as_i64()), Some(9));
        assert_eq!(value.get("trackCount").and_then(|v| v.as_i64()), Some(120));
        assert_eq!(
            value.get("coverKey").and_then(|v| v.as_str()),
            Some("cover-artist-7")
        );
    }
}
