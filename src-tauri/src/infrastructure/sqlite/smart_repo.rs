//! `SmartPlaylistRepository` on SQLite: storage plus the rule compiler
//! (CONTRACTS §1.4, §3).
//!
//! Rules are data. They are compiled into a `WHERE` clause here, with every
//! user-supplied value bound as a parameter — no rule ever contributes text to
//! the statement.

use sqlx::sqlite::SqliteRow;
use sqlx::{QueryBuilder, Row, Sqlite};

use crate::domain::{
    NumOp, SmartPlaylist, SmartPlaylistDraft, SmartRule, SmartSort, Track, MS_PER_DAY,
};
use crate::error::{CoreError, CoreResult};
use crate::infrastructure::repositories::SmartPlaylistRepository;

use super::pool::Db;
use super::sql::{
    dyn_query, like_contains, now_ms, track_order_by, track_select, tracks_from_rows,
    TRACK_IS_VISIBLE,
};

pub struct SqliteSmartPlaylistRepository {
    pool: Db,
}

impl SqliteSmartPlaylistRepository {
    pub fn new(pool: Db) -> Self {
        Self { pool }
    }
}

const SMART_COLUMNS: &str =
    "id, name, rules_json, match_all, sort, limit_n, created_at, updated_at";

fn smart_from_row(row: &SqliteRow) -> CoreResult<SmartPlaylist> {
    let id: i64 = row.try_get("id")?;
    let rules_json: String = row.try_get("rules_json")?;
    let rules: Vec<SmartRule> = serde_json::from_str(&rules_json)?;
    let sort_token: String = row.try_get("sort")?;
    let sort = SmartSort::from_token(&sort_token).ok_or_else(|| {
        CoreError::internal(format!(
            "smart playlist {id} has an unknown sort {sort_token}"
        ))
    })?;

    Ok(SmartPlaylist {
        id,
        name: row.try_get("name")?,
        rules,
        match_all: row.try_get::<i64, _>("match_all")? != 0,
        sort,
        limit: row.try_get("limit_n")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

const fn num_op(op: NumOp) -> &'static str {
    match op {
        NumOp::Eq => " = ",
        NumOp::Ne => " <> ",
        NumOp::Lt => " < ",
        NumOp::Lte => " <= ",
        NumOp::Gt => " > ",
        NumOp::Gte => " >= ",
    }
}

fn days_ago(now: i64, days: i64) -> i64 {
    now.saturating_sub(days.saturating_mul(MS_PER_DAY))
}

/// One rule as a self-contained, parenthesised predicate over the aliases of
/// [`track_select`].
fn push_rule(builder: &mut QueryBuilder<Sqlite>, rule: &SmartRule, now: i64) {
    builder.push("(");
    match rule {
        SmartRule::PlayCount { op, value } => {
            builder.push("COALESCE(pc.count, 0)");
            builder.push(num_op(*op));
            builder.push_bind(*value);
        }
        SmartRule::NeverPlayed => {
            builder.push("COALESCE(pc.count, 0) = 0");
        }
        SmartRule::LastPlayedBeforeDays { days } => {
            builder.push("pc.last_played_at IS NOT NULL AND pc.last_played_at < ");
            builder.push_bind(days_ago(now, *days));
        }
        SmartRule::AddedWithinDays { days } => {
            builder.push("t.date_added >= ");
            builder.push_bind(days_ago(now, *days));
        }
        SmartRule::Favorite { value } => {
            builder.push(if *value {
                "f.track_id IS NOT NULL"
            } else {
                "f.track_id IS NULL"
            });
        }
        SmartRule::YearBetween { from, to } => {
            builder.push("t.year IS NOT NULL AND t.year BETWEEN ");
            builder.push_bind(*from);
            builder.push(" AND ");
            builder.push_bind(*to);
        }
        SmartRule::BitrateAtLeast { value } => {
            builder.push("t.bitrate IS NOT NULL AND t.bitrate >= ");
            builder.push_bind(*value);
        }
        SmartRule::DurationBetweenMs { from, to } => {
            builder.push("t.duration_ms BETWEEN ");
            builder.push_bind(*from);
            builder.push(" AND ");
            builder.push_bind(*to);
        }
        SmartRule::GenreIs { value } => {
            builder.push("t.genre = ");
            builder.push_bind(value.trim().to_owned());
            builder.push(" COLLATE NOCASE");
        }
        SmartRule::ArtistIs { artist_id } => {
            builder.push("t.artist_id = ");
            builder.push_bind(*artist_id);
        }
        SmartRule::TitleContains { value } => {
            builder.push("t.title LIKE ");
            builder.push_bind(like_contains(value.trim()));
            builder.push(" ESCAPE '\\'");
        }
    }
    builder.push(")");
}

#[async_trait::async_trait]
impl SmartPlaylistRepository for SqliteSmartPlaylistRepository {
    async fn list(&self) -> CoreResult<Vec<SmartPlaylist>> {
        let rows = dyn_query(format!(
            "SELECT {SMART_COLUMNS} FROM smart_playlists ORDER BY id ASC"
        ))
        .fetch_all(&self.pool)
        .await?;

        let mut playlists = Vec::with_capacity(rows.len());
        for row in &rows {
            playlists.push(smart_from_row(row)?);
        }
        Ok(playlists)
    }

    async fn get(&self, id: i64) -> CoreResult<SmartPlaylist> {
        let row = dyn_query(format!(
            "SELECT {SMART_COLUMNS} FROM smart_playlists WHERE id = ?"
        ))
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| CoreError::not_found("smart playlist", id))?;
        smart_from_row(&row)
    }

    async fn create(&self, draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylist> {
        let draft = draft.validated()?;
        let rules_json = serde_json::to_string(&draft.rules)?;
        let now = now_ms();

        let created = sqlx::query(
            "INSERT INTO smart_playlists \
               (name, rules_json, match_all, sort, limit_n, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&draft.name)
        .bind(&rules_json)
        .bind(i64::from(draft.match_all))
        .bind(draft.sort.as_str())
        .bind(draft.limit)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.get(created.last_insert_rowid()).await
    }

    async fn update(&self, id: i64, draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylist> {
        let draft = draft.validated()?;
        let rules_json = serde_json::to_string(&draft.rules)?;

        let updated = sqlx::query(
            "UPDATE smart_playlists SET \
               name = ?, rules_json = ?, match_all = ?, sort = ?, limit_n = ?, updated_at = ? \
             WHERE id = ?",
        )
        .bind(&draft.name)
        .bind(&rules_json)
        .bind(i64::from(draft.match_all))
        .bind(draft.sort.as_str())
        .bind(draft.limit)
        .bind(now_ms())
        .bind(id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(CoreError::not_found("smart playlist", id));
        }

        self.get(id).await
    }

    async fn delete(&self, id: i64) -> CoreResult<()> {
        let deleted = sqlx::query("DELETE FROM smart_playlists WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if deleted.rows_affected() == 0 {
            return Err(CoreError::not_found("smart playlist", id));
        }
        Ok(())
    }

    async fn resolve(
        &self,
        rules: &[SmartRule],
        match_all: bool,
        sort: SmartSort,
        limit: Option<i64>,
    ) -> CoreResult<Vec<Track>> {
        for rule in rules {
            rule.validate()?;
        }

        let now = now_ms();
        let mut builder = QueryBuilder::<Sqlite>::new(track_select());
        builder.push(" WHERE ").push(TRACK_IS_VISIBLE);

        if !rules.is_empty() {
            builder.push(" AND (");
            let joiner = if match_all { " AND " } else { " OR " };
            for (index, rule) in rules.iter().enumerate() {
                if index > 0 {
                    builder.push(joiner);
                }
                push_rule(&mut builder, rule, now);
            }
            builder.push(")");
        }

        match sort.as_track_sort() {
            Some(track_sort) => {
                builder.push(" ORDER BY ").push(track_order_by(track_sort));
            }
            None => {
                builder.push(" ORDER BY RANDOM()");
            }
        }

        if let Some(limit) = limit.filter(|value| *value > 0) {
            builder.push(" LIMIT ").push_bind(limit);
        }

        let rows = builder.build().fetch_all(&self.pool).await?;
        Ok(tracks_from_rows(&rows)?)
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteSmartPlaylistRepository;
    use crate::domain::{
        NumOp, ScannedTrack, SmartPlaylistDraft, SmartRule, SmartSort, MS_PER_DAY,
    };
    use crate::infrastructure::repositories::{
        FavoriteRepository, HistoryRepository, SmartPlaylistRepository, TrackRepository,
    };
    use crate::infrastructure::sqlite::favorite_repo::SqliteFavoriteRepository;
    use crate::infrastructure::sqlite::history_repo::SqliteHistoryRepository;
    use crate::infrastructure::sqlite::sql::{now_ms, test_support::pool};
    use crate::infrastructure::sqlite::track_repo::SqliteTrackRepository;

    struct Library {
        smart: SqliteSmartPlaylistRepository,
        tracks: SqliteTrackRepository,
        favorites: SqliteFavoriteRepository,
        history: SqliteHistoryRepository,
        ids: Vec<i64>,
    }

    fn track(uri: &str, title: &str, year: i32, bitrate: i32, genre: &str) -> ScannedTrack {
        ScannedTrack {
            uri: uri.to_owned(),
            title: title.to_owned(),
            artist: Some("Artist".to_owned()),
            album: Some("Album".to_owned()),
            album_artist: None,
            duration_ms: 200_000,
            track_number: None,
            disc_number: None,
            year: Some(year),
            genre: Some(genre.to_owned()),
            bitrate: Some(bitrate),
            sample_rate: Some(44_100),
            size: 5_000_000,
            mime_type: Some("audio/flac".to_owned()),
            folder: Some("Music".to_owned()),
            date_added: now_ms(),
            last_modified: now_ms(),
            cover_key: None,
        }
    }

    /// Three tracks: a recent lossless 2021 rock one, an old 2001 pop one that
    /// was played twice, and a short 1995 jazz one.
    async fn library() -> Library {
        let db = pool().await;
        let tracks = SqliteTrackRepository::new(db.clone());

        let mut old = track("content://old", "Old Song", 2001, 128_000, "Pop");
        old.date_added = now_ms() - 400 * MS_PER_DAY;
        let mut short = track("content://short", "Short Song", 1995, 192_000, "Jazz");
        short.duration_ms = 45_000;
        short.date_added = now_ms() - 400 * MS_PER_DAY;

        tracks
            .upsert_many(&[
                track("content://new", "New Song", 2021, 1_000_000, "Rock"),
                old,
                short,
            ])
            .await
            .expect("library scan");

        let ids: Vec<i64> = tracks
            .query(&crate::domain::TrackQuery::default())
            .await
            .expect("tracks")
            .items
            .iter()
            .map(|t| t.id)
            .collect();

        Library {
            smart: SqliteSmartPlaylistRepository::new(db.clone()),
            tracks,
            favorites: SqliteFavoriteRepository::new(db.clone()),
            history: SqliteHistoryRepository::new(db),
            ids,
        }
    }

    async fn titles(
        repo: &SqliteSmartPlaylistRepository,
        rules: &[SmartRule],
        match_all: bool,
    ) -> Vec<String> {
        repo.resolve(rules, match_all, SmartSort::TitleAsc, None)
            .await
            .expect("resolved")
            .into_iter()
            .map(|track| track.title)
            .collect()
    }

    #[tokio::test]
    async fn seeded_presets_are_readable() {
        let library = library().await;
        let presets = library.smart.list().await.expect("presets");

        assert_eq!(presets.len(), 7);
        assert_eq!(presets[0].name, "Недавно добавленные");
        assert!(!presets[0].rules.is_empty());
        assert_eq!(presets[0].sort, SmartSort::DateAddedDesc);
        assert_eq!(presets[0].limit, Some(100));
        assert_eq!(presets[1].limit, None);

        // Every preset must survive the compiler.
        for preset in &presets {
            library
                .smart
                .resolve(&preset.rules, preset.match_all, preset.sort, preset.limit)
                .await
                .expect("preset resolves");
        }
    }

    #[tokio::test]
    async fn crud_round_trip() {
        let library = library().await;
        let draft = SmartPlaylistDraft {
            name: "  Loud  ".to_owned(),
            rules: vec![SmartRule::BitrateAtLeast { value: 320_000 }],
            match_all: true,
            sort: SmartSort::YearDesc,
            limit: Some(10),
        };

        let created = library.smart.create(&draft).await.expect("created");
        assert_eq!(created.name, "Loud");
        assert_eq!(created.rules, draft.rules);
        assert_eq!(created.sort, SmartSort::YearDesc);
        assert_eq!(created.limit, Some(10));
        assert!(created.created_at > 0);

        let fetched = library.smart.get(created.id).await.expect("fetched");
        assert_eq!(fetched, created);

        let updated = library
            .smart
            .update(
                created.id,
                &SmartPlaylistDraft {
                    name: "Quiet".to_owned(),
                    rules: vec![SmartRule::NeverPlayed],
                    match_all: false,
                    sort: SmartSort::Random,
                    limit: None,
                },
            )
            .await
            .expect("updated");
        assert_eq!(updated.name, "Quiet");
        assert_eq!(updated.rules, vec![SmartRule::NeverPlayed]);
        assert!(!updated.match_all);
        assert_eq!(updated.sort, SmartSort::Random);
        assert_eq!(updated.limit, None);

        library.smart.delete(created.id).await.expect("deleted");
        assert!(library
            .smart
            .get(created.id)
            .await
            .expect_err("gone")
            .is_not_found());
        assert!(library
            .smart
            .delete(created.id)
            .await
            .expect_err("gone")
            .is_not_found());
        assert!(library
            .smart
            .update(created.id, &draft)
            .await
            .expect_err("gone")
            .is_not_found());
    }

    #[tokio::test]
    async fn invalid_rules_are_rejected_before_they_reach_sql() {
        let library = library().await;
        let error = library
            .smart
            .resolve(
                &[SmartRule::YearBetween {
                    from: 2020,
                    to: 1990,
                }],
                true,
                SmartSort::TitleAsc,
                None,
            )
            .await
            .expect_err("contradictory range");
        assert_eq!(error.code(), "INVALID_INPUT");
    }

    #[tokio::test]
    async fn match_all_ands_and_match_any_ors() {
        let library = library().await;
        let rules = vec![
            SmartRule::YearBetween {
                from: 2020,
                to: 2029,
            },
            SmartRule::GenreIs {
                value: "jazz".to_owned(),
            },
        ];

        assert!(titles(&library.smart, &rules, true).await.is_empty());
        assert_eq!(
            titles(&library.smart, &rules, false).await,
            vec!["New Song", "Short Song"]
        );
    }

    #[tokio::test]
    async fn every_rule_kind_compiles_and_selects() {
        let library = library().await;

        assert_eq!(
            titles(
                &library.smart,
                &[SmartRule::AddedWithinDays { days: 30 }],
                true
            )
            .await,
            vec!["New Song"]
        );
        assert_eq!(
            titles(
                &library.smart,
                &[SmartRule::BitrateAtLeast { value: 320_000 }],
                true
            )
            .await,
            vec!["New Song"]
        );
        assert_eq!(
            titles(
                &library.smart,
                &[SmartRule::DurationBetweenMs {
                    from: 0,
                    to: 60_000
                }],
                true
            )
            .await,
            vec!["Short Song"]
        );
        assert_eq!(
            titles(
                &library.smart,
                &[SmartRule::TitleContains {
                    value: "old".to_owned()
                }],
                true
            )
            .await,
            vec!["Old Song"]
        );
        assert_eq!(
            titles(&library.smart, &[SmartRule::NeverPlayed], true)
                .await
                .len(),
            3
        );

        let artist_id = library
            .tracks
            .get(library.ids[0])
            .await
            .expect("track")
            .artist_id
            .expect("artist");
        assert_eq!(
            titles(&library.smart, &[SmartRule::ArtistIs { artist_id }], true)
                .await
                .len(),
            3
        );
    }

    #[tokio::test]
    async fn play_statistics_rules_follow_the_history() {
        let library = library().await;
        let played = library.ids[1];
        let now = now_ms();

        library
            .history
            .record(played, now - 90 * MS_PER_DAY, 200_000)
            .await
            .expect("old play");

        assert_eq!(
            titles(&library.smart, &[SmartRule::NeverPlayed], true)
                .await
                .len(),
            2
        );
        assert_eq!(
            titles(
                &library.smart,
                &[SmartRule::PlayCount {
                    op: NumOp::Gte,
                    value: 1
                }],
                true
            )
            .await
            .len(),
            1
        );
        assert_eq!(
            titles(
                &library.smart,
                &[SmartRule::LastPlayedBeforeDays { days: 60 }],
                true
            )
            .await
            .len(),
            1,
            "a play from 90 days ago is forgotten"
        );
        assert!(titles(
            &library.smart,
            &[SmartRule::LastPlayedBeforeDays { days: 120 }],
            true
        )
        .await
        .is_empty());
    }

    #[tokio::test]
    async fn favorite_rule_reads_the_favorites_table() {
        let library = library().await;
        library
            .favorites
            .toggle(library.ids[0])
            .await
            .expect("favorited");

        assert_eq!(
            titles(&library.smart, &[SmartRule::Favorite { value: true }], true)
                .await
                .len(),
            1
        );
        assert_eq!(
            titles(
                &library.smart,
                &[SmartRule::Favorite { value: false }],
                true
            )
            .await
            .len(),
            2
        );
    }

    #[tokio::test]
    async fn wildcards_in_a_title_rule_are_literal() {
        let library = library().await;
        assert!(titles(
            &library.smart,
            &[SmartRule::TitleContains {
                value: "%Song".to_owned()
            }],
            true
        )
        .await
        .is_empty());
    }

    #[tokio::test]
    async fn sorting_and_limit_apply() {
        let library = library().await;
        let rules = vec![SmartRule::DurationBetweenMs {
            from: 0,
            to: 10_000_000,
        }];

        let newest = library
            .smart
            .resolve(&rules, true, SmartSort::YearDesc, Some(1))
            .await
            .expect("resolved");
        assert_eq!(newest.len(), 1);
        assert_eq!(newest[0].title, "New Song");

        let random = library
            .smart
            .resolve(&rules, true, SmartSort::Random, None)
            .await
            .expect("resolved");
        assert_eq!(random.len(), 3, "RANDOM() must not drop rows");

        let all = library
            .smart
            .resolve(&[], true, SmartSort::TitleAsc, None)
            .await
            .expect("no rules");
        assert_eq!(all.len(), 3, "a rule-less set is the whole library");
    }
}
