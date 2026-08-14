//! `TaxonomyRepository` on SQLite: genres and the folder tree (CONTRACTS §3).

use std::collections::BTreeMap;

use sqlx::Row;

use crate::domain::{Folder, Genre, FOLDER_SEPARATOR};
use crate::error::CoreResult;
use crate::infrastructure::repositories::TaxonomyRepository;

use super::pool::Db;

pub struct SqliteTaxonomyRepository {
    pool: Db,
}

impl SqliteTaxonomyRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }
}

/// Genres differing only in case are one genre; `MIN` picks the spelling shown
/// to the user deterministically instead of leaving it to the group scan.
const GENRES_SQL: &str = "SELECT MIN(TRIM(genre)) AS name, COUNT(*) AS track_count \
     FROM tracks t \
     WHERE genre IS NOT NULL AND TRIM(genre) <> '' \
       AND NOT EXISTS (SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id) \
     GROUP BY TRIM(genre) COLLATE NOCASE \
     ORDER BY name COLLATE NOCASE ASC";

/// Distinct folder paths with the number of tracks sitting directly in them.
const FOLDERS_SQL: &str = "SELECT folder AS path, COUNT(*) AS track_count \
     FROM tracks t \
     WHERE folder IS NOT NULL AND TRIM(folder) <> '' \
       AND NOT EXISTS (SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id) \
     GROUP BY folder";

#[async_trait::async_trait]
impl TaxonomyRepository for SqliteTaxonomyRepository {
    async fn genres(&self) -> CoreResult<Vec<Genre>> {
        let rows = sqlx::query(GENRES_SQL).fetch_all(&self.pool).await?;
        let mut genres = Vec::with_capacity(rows.len());
        for row in &rows {
            genres.push(Genre {
                name: row.try_get("name")?,
                track_count: row.try_get("track_count")?,
            });
        }
        Ok(genres)
    }

    /// Direct children of `parent` (top level when `None`), each counting every
    /// track in its subtree.
    ///
    /// The grouping happens in Rust: `tracks.folder` holds a whole display path,
    /// and the number of distinct paths is tiny compared to the number of
    /// tracks, so one grouped read beats a pile of string surgery in SQL.
    async fn folders(&self, parent: Option<&str>) -> CoreResult<Vec<Folder>> {
        let rows = sqlx::query(FOLDERS_SQL).fetch_all(&self.pool).await?;

        let prefix = parent
            .map(|value| value.trim_end_matches(FOLDER_SEPARATOR))
            .filter(|value| !value.is_empty());

        let mut children: BTreeMap<String, i64> = BTreeMap::new();
        for row in &rows {
            let path: String = row.try_get("path")?;
            let count: i64 = row.try_get("track_count")?;
            let Some(child) = child_path(path.trim().trim_end_matches(FOLDER_SEPARATOR), prefix)
            else {
                continue;
            };
            *children.entry(child).or_insert(0) += count;
        }

        let mut folders: Vec<Folder> = children
            .into_iter()
            .map(|(path, count)| Folder::from_path(path, count))
            .collect();
        folders.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.path.cmp(&b.path))
        });
        Ok(folders)
    }
}

/// The child of `parent` that `path` belongs to, or `None` when `path` is not
/// inside `parent`. With no parent this is the first segment of the path.
fn child_path(path: &str, parent: Option<&str>) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let rest = match parent {
        Some(parent) => {
            let tail = path.strip_prefix(parent)?;
            tail.strip_prefix(FOLDER_SEPARATOR)?
        }
        None => path,
    };
    let segment = rest.split(FOLDER_SEPARATOR).next().unwrap_or_default();
    if segment.is_empty() {
        return None;
    }
    Some(match parent {
        Some(parent) => format!("{parent}{FOLDER_SEPARATOR}{segment}"),
        None => segment.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{child_path, SqliteTaxonomyRepository};
    use crate::domain::ScannedTrack;
    use crate::infrastructure::repositories::{TaxonomyRepository, TrackRepository};
    use crate::infrastructure::sqlite::sql::test_support::{pool, scanned};
    use crate::infrastructure::sqlite::track_repo::SqliteTrackRepository;

    fn in_folder(uri: &str, folder: &str, genre: Option<&str>) -> ScannedTrack {
        ScannedTrack {
            folder: Some(folder.to_owned()),
            genre: genre.map(str::to_owned),
            ..scanned(uri, uri)
        }
    }

    async fn seeded() -> SqliteTaxonomyRepository {
        let db = pool().await;
        let tracks = SqliteTrackRepository::new(db.clone());
        tracks
            .upsert_many(&[
                in_folder("content://1", "Music/Rock", Some("Rock")),
                in_folder("content://2", "Music/Rock/Live", Some("rock")),
                in_folder("content://3", "Music/Jazz", Some("Jazz")),
                in_folder("content://4", "Podcasts", None),
                in_folder("content://5", "Podcasts", Some("  ")),
            ])
            .await
            .expect("library scan");
        SqliteTaxonomyRepository::new(db)
    }

    #[tokio::test]
    async fn genres_are_case_insensitive_and_skip_blanks() {
        let taxonomy = seeded().await;
        let genres = taxonomy.genres().await.expect("genres");

        let names: Vec<&str> = genres.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, vec!["Jazz", "Rock"]);
        let rock = genres.iter().find(|g| g.name == "Rock").expect("rock");
        assert_eq!(rock.track_count, 2, "Rock and rock are one genre");
    }

    #[tokio::test]
    async fn top_level_folders_count_their_subtree() {
        let taxonomy = seeded().await;
        let folders = taxonomy.folders(None).await.expect("folders");

        let paths: Vec<&str> = folders.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["Music", "Podcasts"]);
        assert_eq!(folders[0].track_count, 3);
        assert_eq!(folders[0].name, "Music");
        assert_eq!(folders[1].track_count, 2);
    }

    #[tokio::test]
    async fn children_are_direct_descendants_only() {
        let taxonomy = seeded().await;

        let music = taxonomy.folders(Some("Music")).await.expect("children");
        let paths: Vec<&str> = music.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["Music/Jazz", "Music/Rock"]);
        let rock = music.iter().find(|f| f.name == "Rock").expect("rock");
        assert_eq!(rock.track_count, 2, "the Live subtree counts towards Rock");

        let live = taxonomy
            .folders(Some("Music/Rock"))
            .await
            .expect("grandchildren");
        assert_eq!(live.len(), 1);
        assert_eq!(live[0].path, "Music/Rock/Live");
        assert_eq!(live[0].track_count, 1);

        assert!(taxonomy
            .folders(Some("Music/Rock/Live"))
            .await
            .expect("leaf")
            .is_empty());
        assert!(taxonomy
            .folders(Some("Nowhere"))
            .await
            .expect("unknown")
            .is_empty());
    }

    #[test]
    fn child_paths_do_not_leak_across_siblings() {
        assert_eq!(child_path("Music/Rock", None).as_deref(), Some("Music"));
        assert_eq!(
            child_path("Music/Rock/Live", Some("Music")).as_deref(),
            Some("Music/Rock")
        );
        assert_eq!(child_path("Music", Some("Music")), None);
        assert_eq!(child_path("MusicVideos/Clip", Some("Music")), None);
        assert_eq!(child_path("", None), None);
    }
}
