//! Connection pool and migrations (CONTRACTS §4).

use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::migrate::Migrator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;

use crate::error::{CoreError, CoreResult};

/// The one handle the repositories are built from.
pub type Db = SqlitePool;

/// Embedded at compile time from `migrations/`; every file is applied in
/// version order and never edited afterwards.
static MIGRATOR: Migrator = sqlx::migrate!("./src/infrastructure/sqlite/migrations");

/// Default file name inside the app data directory.
pub const DB_FILE_NAME: &str = "library.db";

/// A busy database is a database another connection is writing to. WAL makes
/// that rare, this makes it survivable.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_CONNECTIONS: u32 = 5;

/// Opens (creating it when needed) the library database at `path` and brings
/// the schema up to date.
pub async fn connect(path: impl AsRef<Path>) -> CoreResult<Db> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(BUSY_TIMEOUT);

    let pool = SqlitePoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .acquire_timeout(ACQUIRE_TIMEOUT)
        .connect_with(options)
        .await?;

    migrate(&pool).await?;
    Ok(pool)
}

/// Private in-memory database, one connection so the schema stays visible.
/// Used by the repository tests.
pub async fn connect_in_memory() -> CoreResult<Db> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .map_err(CoreError::from)?
        .foreign_keys(true)
        .busy_timeout(BUSY_TIMEOUT);

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(None::<Duration>)
        .max_lifetime(None::<Duration>)
        .connect_with(options)
        .await?;

    migrate(&pool).await?;
    Ok(pool)
}

async fn migrate(pool: &Db) -> CoreResult<()> {
    MIGRATOR.run(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::connect_in_memory;
    use sqlx::Row;

    #[tokio::test]
    async fn migrations_create_the_whole_schema() {
        let pool = connect_in_memory().await.expect("in-memory database");

        let names: Vec<String> =
            sqlx::query("SELECT name FROM sqlite_master WHERE type IN ('table') ORDER BY name")
                .fetch_all(&pool)
                .await
                .expect("schema is readable")
                .iter()
                .map(|row| row.get::<String, _>("name"))
                .collect();

        for expected in [
            "albums",
            "artists",
            "favorites",
            "history",
            "play_counts",
            "playlist_tracks",
            "playlists",
            "smart_playlists",
            "tracks",
            "tracks_fts",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "missing table {expected} in {names:?}"
            );
        }
    }

    #[tokio::test]
    async fn indexes_are_created() {
        let pool = connect_in_memory().await.expect("in-memory database");
        let names: Vec<String> = sqlx::query("SELECT name FROM sqlite_master WHERE type = 'index'")
            .fetch_all(&pool)
            .await
            .expect("schema is readable")
            .iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();

        for expected in [
            "idx_tracks_artist",
            "idx_tracks_album",
            "idx_tracks_date_added",
            "idx_tracks_sort_title",
            "idx_tracks_genre",
            "idx_tracks_folder",
            "idx_playlist_tracks_track",
            "idx_history_played_at",
            "idx_history_track",
            "idx_play_counts_count",
        ] {
            assert!(
                names.iter().any(|name| name == expected),
                "missing index {expected}"
            );
        }
    }

    #[tokio::test]
    async fn preset_smart_playlists_are_seeded() {
        let pool = connect_in_memory().await.expect("in-memory database");
        let rows = sqlx::query("SELECT name, rules_json, sort FROM smart_playlists ORDER BY id")
            .fetch_all(&pool)
            .await
            .expect("presets are readable");

        let names: Vec<String> = rows
            .iter()
            .map(|row| row.get::<String, _>("name"))
            .collect();
        assert_eq!(
            names,
            vec![
                "Недавно добавленные",
                "Ни разу не играли",
                "Забытые",
                "Избранное",
                "Часто слушаю",
                "2020-е",
                "Высокое качество",
            ]
        );

        for row in &rows {
            let json: String = row.get("rules_json");
            let rules: Vec<crate::domain::SmartRule> =
                serde_json::from_str(&json).expect("preset rules parse");
            assert!(!rules.is_empty(), "preset {json} has no rules");
            for rule in &rules {
                rule.validate().expect("preset rule is valid");
            }
            let sort: String = row.get("sort");
            assert!(
                crate::domain::SmartSort::from_token(&sort).is_some(),
                "preset sort {sort} is not a SmartSort token"
            );
        }
    }

    #[tokio::test]
    async fn foreign_keys_are_enforced() {
        let pool = connect_in_memory().await.expect("in-memory database");
        let result = sqlx::query("INSERT INTO favorites (track_id, created_at) VALUES (404, 0)")
            .execute(&pool)
            .await;
        assert!(
            result.is_err(),
            "favorites must reference an existing track"
        );
    }
}
