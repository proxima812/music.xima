//! Shared SQL fragments, row mappers and small helpers.
//!
//! Everything that more than one repository needs lives here so the column
//! lists stay in one place. No repository builds a `Track` on its own.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::query::Query;
use sqlx::sqlite::{SqliteArguments, SqliteConnection, SqliteRow};
use sqlx::{AssertSqlSafe, QueryBuilder, Row, Sqlite};

use crate::domain::{Album, Artist, Playlist, Track, TrackSort, TrackSortField};
use crate::error::{CoreError, CoreResult};

use super::pool::Db;

/// `sqlx::query` for statements assembled at runtime.
///
/// sqlx 0.9 requires SQL strings to be `SqlSafeStr`, which `&str` deliberately
/// is not. Every user-supplied value in this module is bound as a parameter —
/// only the static skeleton (column lists, joins, `ORDER BY`) is interpolated,
/// so the assertion is made here once instead of at each call site.
pub fn dyn_query(sql: String) -> Query<'static, Sqlite, SqliteArguments> {
    sqlx::query(AssertSqlSafe(sql))
}

/// Current wall-clock time in Unix milliseconds. A clock before the epoch
/// yields a negative value instead of panicking.
pub fn now_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(elapsed) => i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX),
        Err(err) => i64::try_from(err.duration().as_millis())
            .map(|ms| -ms)
            .unwrap_or(i64::MIN),
    }
}

/// How many values are bound in one `IN (...)` list. SQLite accepts far more,
/// this only keeps single statements small.
pub const BIND_CHUNK: usize = 500;

/// A history row counts as a play when the listener stayed at least this long.
pub const MIN_PLAY_MS: i64 = 30_000;

/// Predicate deciding whether a `history` row counts as a play: half a minute,
/// or half the track. Expects the `h` (history) and `t` (tracks) aliases.
pub const COUNTS_AS_PLAY: &str = "(h.duration_played_ms >= 30000 \
     OR (t.duration_ms > 0 AND h.duration_played_ms * 2 >= t.duration_ms))";

/// Visibility predicate for queries that alias `tracks` as `t`.
pub const TRACK_IS_VISIBLE: &str = "NOT EXISTS (\
     SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id\
     )";

/// Every column of [`Track`], resolved in a single pass. Requires the
/// `t`/`ar`/`al`/`f`/`pc` aliases from [`TRACK_JOINS`].
pub const TRACK_COLUMNS: &str = "t.id AS id, \
     t.uri AS uri, \
     t.title AS title, \
     t.artist_id AS artist_id, \
     ar.name AS artist_name, \
     t.album_id AS album_id, \
     al.title AS album_title, \
     t.duration_ms AS duration_ms, \
     t.track_number AS track_number, \
     t.disc_number AS disc_number, \
     t.year AS year, \
     t.genre AS genre, \
     t.bitrate AS bitrate, \
     t.sample_rate AS sample_rate, \
     t.size AS size, \
     t.format AS format, \
     COALESCE(t.cover_key, al.cover_key) AS cover_key, \
     t.folder AS folder, \
     t.date_added AS date_added, \
     t.last_modified AS last_modified, \
     CASE WHEN f.track_id IS NULL THEN 0 ELSE 1 END AS is_favorite, \
     COALESCE(pc.count, 0) AS play_count, \
     pc.last_played_at AS last_played_at";

/// Joins behind [`TRACK_COLUMNS`]. Kept separate from the `FROM` clause so
/// playlist and search queries can start from their own table.
pub const TRACK_JOINS: &str = "LEFT JOIN artists ar ON ar.id = t.artist_id \
     LEFT JOIN albums al ON al.id = t.album_id \
     LEFT JOIN favorites f ON f.track_id = t.id \
     LEFT JOIN play_counts pc ON pc.track_id = t.id";

/// `SELECT <track columns> FROM tracks t <joins>`.
pub fn track_select() -> String {
    format!("SELECT {TRACK_COLUMNS} FROM tracks t {TRACK_JOINS}")
}

/// Aggregated album row. Callers append their own `WHERE`, then
/// `GROUP BY al.id`.
pub const ALBUM_SELECT: &str = "SELECT al.id AS id, \
     al.title AS title, \
     al.artist_id AS artist_id, \
     ar.name AS artist_name, \
     al.year AS year, \
     COALESCE(al.cover_key, MIN(t.cover_key)) AS cover_key, \
     COUNT(t.id) AS track_count, \
     COALESCE(SUM(t.duration_ms), 0) AS duration_ms \
     FROM albums al \
     LEFT JOIN artists ar ON ar.id = al.artist_id \
     JOIN tracks t ON t.album_id = al.id AND NOT EXISTS (\
       SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id\
     )";

/// Aggregated artist row. Counts come from correlated subqueries so callers can
/// add a plain `WHERE` without worrying about grouping.
pub const ARTIST_SELECT: &str = "SELECT ar.id AS id, \
     ar.name AS name, \
     (SELECT COUNT(DISTINCT t.album_id) FROM tracks t \
        WHERE t.artist_id = ar.id AND t.album_id IS NOT NULL AND NOT EXISTS (\
          SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id\
        )) AS album_count, \
     (SELECT COUNT(*) FROM tracks t WHERE t.artist_id = ar.id AND NOT EXISTS (\
        SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id\
     )) AS track_count, \
     (SELECT COALESCE(t.cover_key, al.cover_key) FROM tracks t \
        LEFT JOIN albums al ON al.id = t.album_id \
        WHERE t.artist_id = ar.id AND COALESCE(t.cover_key, al.cover_key) IS NOT NULL \
          AND NOT EXISTS (SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id) \
        LIMIT 1) AS cover_key \
     FROM artists ar";

/// Aggregated playlist row. Callers append their own `WHERE`, then
/// `GROUP BY p.id`.
pub const PLAYLIST_SELECT: &str = "SELECT p.id AS id, \
     p.name AS name, \
     COUNT(t.id) AS track_count, \
     COALESCE(SUM(t.duration_ms), 0) AS duration_ms, \
     (SELECT COALESCE(t2.cover_key, al2.cover_key) FROM playlist_tracks pt2 \
        JOIN tracks t2 ON t2.id = pt2.track_id \
        LEFT JOIN albums al2 ON al2.id = t2.album_id \
        WHERE pt2.playlist_id = p.id \
          AND NOT EXISTS (SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t2.id) \
          AND COALESCE(t2.cover_key, al2.cover_key) IS NOT NULL \
        ORDER BY pt2.position ASC LIMIT 1) AS cover_key, \
     p.created_at AS created_at, \
     p.updated_at AS updated_at \
     FROM playlists p \
     LEFT JOIN playlist_tracks pt ON pt.playlist_id = p.id \
     LEFT JOIN tracks t ON t.id = pt.track_id AND NOT EXISTS (\
       SELECT 1 FROM hidden_tracks hidden WHERE hidden.track_id = t.id\
     )";

pub fn track_from_row(row: &SqliteRow) -> Result<Track, sqlx::Error> {
    Ok(Track {
        id: row.try_get("id")?,
        uri: row.try_get("uri")?,
        title: row.try_get("title")?,
        artist_id: row.try_get("artist_id")?,
        artist_name: row.try_get("artist_name")?,
        album_id: row.try_get("album_id")?,
        album_title: row.try_get("album_title")?,
        duration_ms: row.try_get("duration_ms")?,
        track_number: row.try_get("track_number")?,
        disc_number: row.try_get("disc_number")?,
        year: row.try_get("year")?,
        genre: row.try_get("genre")?,
        bitrate: row.try_get("bitrate")?,
        sample_rate: row.try_get("sample_rate")?,
        size: row.try_get("size")?,
        format: row.try_get("format")?,
        cover_key: row.try_get("cover_key")?,
        folder: row.try_get("folder")?,
        date_added: row.try_get("date_added")?,
        last_modified: row.try_get("last_modified")?,
        is_favorite: row.try_get::<i64, _>("is_favorite")? != 0,
        play_count: row.try_get("play_count")?,
        last_played_at: row.try_get("last_played_at")?,
    })
}

/// [`track_from_row`] over a whole result set.
pub fn tracks_from_rows(rows: &[SqliteRow]) -> Result<Vec<Track>, sqlx::Error> {
    let mut tracks = Vec::with_capacity(rows.len());
    for row in rows {
        tracks.push(track_from_row(row)?);
    }
    Ok(tracks)
}

pub fn album_from_row(row: &SqliteRow) -> Result<Album, sqlx::Error> {
    Ok(Album {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        artist_id: row.try_get("artist_id")?,
        artist_name: row.try_get("artist_name")?,
        year: row.try_get("year")?,
        cover_key: row.try_get("cover_key")?,
        track_count: row.try_get("track_count")?,
        duration_ms: row.try_get("duration_ms")?,
    })
}

pub fn artist_from_row(row: &SqliteRow) -> Result<Artist, sqlx::Error> {
    Ok(Artist {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        album_count: row.try_get("album_count")?,
        track_count: row.try_get("track_count")?,
        cover_key: row.try_get("cover_key")?,
    })
}

pub fn playlist_from_row(row: &SqliteRow) -> Result<Playlist, sqlx::Error> {
    Ok(Playlist {
        id: row.try_get("id")?,
        name: row.try_get("name")?,
        track_count: row.try_get("track_count")?,
        duration_ms: row.try_get("duration_ms")?,
        cover_key: row.try_get("cover_key")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

/// `ORDER BY` body for a library sort. Built from the enum alone, so nothing
/// user-supplied ever reaches the statement text.
pub fn track_order_by(sort: TrackSort) -> String {
    let direction = if sort.descending() { "DESC" } else { "ASC" };
    let primary = match sort.field() {
        TrackSortField::Title => "t.sort_title",
        TrackSortField::Artist => "ar.sort_name",
        TrackSortField::Album => "al.sort_title",
        TrackSortField::DateAdded => "t.date_added",
        TrackSortField::Duration => "t.duration_ms",
        TrackSortField::PlayCount => "COALESCE(pc.count, 0)",
        TrackSortField::LastPlayed => "pc.last_played_at",
        TrackSortField::Year => "t.year",
    };
    let tail = match sort.field() {
        TrackSortField::Title => "t.id ASC",
        TrackSortField::Artist => {
            "al.sort_title ASC, t.disc_number ASC, t.track_number ASC, t.sort_title ASC, t.id ASC"
        }
        TrackSortField::Album => {
            "t.disc_number ASC, t.track_number ASC, t.sort_title ASC, t.id ASC"
        }
        _ => "t.sort_title ASC, t.id ASC",
    };
    format!("{primary} {direction}, {tail}")
}

/// Escape character used with every `LIKE` in this module.
const LIKE_ESCAPE: char = '\\';

/// Turns raw user input into a `%…%` pattern with the wildcards neutralised.
/// Always paired with `ESCAPE '\'` in the statement.
pub fn like_contains(raw: &str) -> String {
    let mut pattern = String::with_capacity(raw.len() + 2);
    pattern.push('%');
    for ch in raw.chars() {
        if ch == '%' || ch == '_' || ch == LIKE_ESCAPE {
            pattern.push(LIKE_ESCAPE);
        }
        pattern.push(ch);
    }
    pattern.push('%');
    pattern
}

/// Compiles a user query into an FTS5 MATCH expression.
///
/// The input is split on everything that is not a letter or a digit, so quotes,
/// `*`, `NEAR`, `-` and the rest of the FTS syntax can never leak in. Each token
/// becomes a quoted prefix term, tokens are ANDed. `None` when nothing is left
/// to search for.
pub fn fts_match(raw: &str) -> Option<String> {
    let mut expression = String::new();
    for token in raw.split(|ch: char| !ch.is_alphanumeric()) {
        if token.is_empty() {
            continue;
        }
        if !expression.is_empty() {
            expression.push(' ');
        }
        expression.push('"');
        expression.push_str(token);
        expression.push_str("\"*");
    }
    if expression.is_empty() {
        None
    } else {
        Some(expression)
    }
}

/// The four indexed values of one `tracks_fts` row.
///
/// `content=''` means FTS5 stores no copy of the text, so a row can only be
/// deleted by replaying the exact values it was inserted with. They are always
/// read back through [`fts_payload`], never reconstructed by hand.
pub struct FtsRow {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
}

const FTS_PAYLOAD_SQL: &str = "SELECT t.id AS id, \
     t.title AS title, \
     COALESCE(ar.name, '') AS artist, \
     COALESCE(al.title, '') AS album, \
     COALESCE(t.genre, '') AS genre \
     FROM tracks t \
     LEFT JOIN artists ar ON ar.id = t.artist_id \
     LEFT JOIN albums al ON al.id = t.album_id \
     WHERE t.id = ?";

/// Values currently indexed for `track_id`, read from the source tables.
pub async fn fts_payload(conn: &mut SqliteConnection, track_id: i64) -> CoreResult<Option<FtsRow>> {
    let row = sqlx::query(FTS_PAYLOAD_SQL)
        .bind(track_id)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(FtsRow {
        id: row.try_get("id")?,
        title: row.try_get("title")?,
        artist: row.try_get("artist")?,
        album: row.try_get("album")?,
        genre: row.try_get("genre")?,
    }))
}

pub async fn fts_insert(conn: &mut SqliteConnection, row: &FtsRow) -> CoreResult<()> {
    sqlx::query(
        "INSERT INTO tracks_fts (rowid, title, artist, album, genre) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(row.id)
    .bind(&row.title)
    .bind(&row.artist)
    .bind(&row.album)
    .bind(&row.genre)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

pub async fn fts_delete(conn: &mut SqliteConnection, row: &FtsRow) -> CoreResult<()> {
    sqlx::query(
        "INSERT INTO tracks_fts (tracks_fts, rowid, title, artist, album, genre) \
         VALUES ('delete', ?, ?, ?, ?, ?)",
    )
    .bind(row.id)
    .bind(&row.title)
    .bind(&row.artist)
    .bind(&row.album)
    .bind(&row.genre)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Tracks for `ids`, in the order they were asked for. Missing ids are skipped.
pub async fn tracks_by_ids(pool: &Db, ids: &[i64]) -> CoreResult<Vec<Track>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut found: HashMap<i64, Track> = HashMap::with_capacity(ids.len());
    for chunk in ids.chunks(BIND_CHUNK) {
        let mut builder =
            QueryBuilder::<Sqlite>::new(format!("{} WHERE t.id IN (", track_select()));
        {
            let mut separated = builder.separated(", ");
            for id in chunk {
                separated.push_bind(*id);
            }
        }
        builder.push(")");
        builder.push(" AND ").push(TRACK_IS_VISIBLE);
        for row in builder.build().fetch_all(pool).await? {
            let track = track_from_row(&row)?;
            found.insert(track.id, track);
        }
    }

    let mut ordered = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(track) = found.get(id) {
            ordered.push(track.clone());
        }
    }
    Ok(ordered)
}

/// `NotFound` when the track is unknown. Every write path that references a
/// track checks this first, so a stale frontend id is a `NOT_FOUND` and not a
/// foreign-key `DATABASE` error.
pub async fn ensure_track(conn: &mut SqliteConnection, track_id: i64) -> CoreResult<()> {
    ensure_row(conn, "SELECT 1 FROM tracks WHERE id = ?", track_id, "track").await
}

/// `NotFound` when the playlist is unknown.
pub async fn ensure_playlist(conn: &mut SqliteConnection, playlist_id: i64) -> CoreResult<()> {
    ensure_row(
        conn,
        "SELECT 1 FROM playlists WHERE id = ?",
        playlist_id,
        "playlist",
    )
    .await
}

async fn ensure_row(
    conn: &mut SqliteConnection,
    statement: &'static str,
    id: i64,
    entity: &str,
) -> CoreResult<()> {
    let found = sqlx::query(statement)
        .bind(id)
        .fetch_optional(&mut *conn)
        .await?;
    if found.is_none() {
        return Err(CoreError::not_found(entity, id));
    }
    Ok(())
}

/// Emits `" WHERE "` the first time and `" AND "` afterwards.
pub fn conjunction(has_where: &mut bool) -> &'static str {
    if *has_where {
        " AND "
    } else {
        *has_where = true;
        " WHERE "
    }
}

#[cfg(test)]
pub mod test_support {
    use crate::domain::ScannedTrack;

    use super::super::pool::{connect_in_memory, Db};

    pub async fn pool() -> Db {
        connect_in_memory().await.expect("in-memory database")
    }

    /// Minimal scanned track; tests tweak the fields they care about.
    pub fn scanned(uri: &str, title: &str) -> ScannedTrack {
        ScannedTrack {
            uri: uri.to_owned(),
            title: title.to_owned(),
            artist: None,
            album: None,
            album_artist: None,
            duration_ms: 180_000,
            track_number: None,
            disc_number: None,
            year: None,
            genre: None,
            bitrate: None,
            sample_rate: None,
            size: 4_000_000,
            mime_type: Some("audio/mpeg".to_owned()),
            folder: None,
            date_added: 1_700_000_000_000,
            last_modified: 1_700_000_000_000,
            cover_key: None,
        }
    }

    pub fn full(uri: &str, title: &str, artist: &str, album: &str) -> ScannedTrack {
        ScannedTrack {
            artist: Some(artist.to_owned()),
            album: Some(album.to_owned()),
            album_artist: Some(artist.to_owned()),
            genre: Some("Rock".to_owned()),
            year: Some(2021),
            bitrate: Some(320_000),
            folder: Some("Music/Rock".to_owned()),
            ..scanned(uri, title)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{fts_match, like_contains, now_ms, track_order_by};
    use crate::domain::TrackSort;

    #[test]
    fn now_is_a_plausible_unix_millisecond_stamp() {
        let now = now_ms();
        assert!(now > 1_700_000_000_000, "clock looks wrong: {now}");
        assert!(now < 4_000_000_000_000, "clock looks wrong: {now}");
    }

    #[test]
    fn like_patterns_neutralise_wildcards() {
        assert_eq!(like_contains("rock"), "%rock%");
        assert_eq!(like_contains("50%_off"), "%50\\%\\_off%");
        assert_eq!(like_contains("back\\slash"), "%back\\\\slash%");
        assert_eq!(like_contains(""), "%%");
    }

    #[test]
    fn fts_queries_are_quoted_prefix_terms() {
        assert_eq!(
            fts_match("pink floyd").as_deref(),
            Some("\"pink\"* \"floyd\"*")
        );
        assert_eq!(
            fts_match("  the   wall ").as_deref(),
            Some("\"the\"* \"wall\"*")
        );
        assert_eq!(fts_match("Аквариум").as_deref(), Some("\"Аквариум\"*"));
    }

    #[test]
    fn fts_queries_strip_operators_and_quotes() {
        assert_eq!(
            fts_match("\"rock\" OR *").as_deref(),
            Some("\"rock\"* \"OR\"*")
        );
        assert_eq!(
            fts_match("a-ha NEAR/2").as_deref(),
            Some("\"a\"* \"ha\"* \"NEAR\"* \"2\"*")
        );
        assert_eq!(fts_match("   "), None);
        assert_eq!(fts_match("\"\"*"), None);
        assert_eq!(fts_match("()"), None);
    }

    #[test]
    fn order_by_uses_known_aliases_only() {
        for sort in [
            TrackSort::TitleAsc,
            TrackSort::TitleDesc,
            TrackSort::ArtistAsc,
            TrackSort::AlbumAsc,
            TrackSort::DateAddedDesc,
            TrackSort::DateAddedAsc,
            TrackSort::DurationAsc,
            TrackSort::DurationDesc,
            TrackSort::PlayCountDesc,
            TrackSort::LastPlayedDesc,
            TrackSort::YearDesc,
        ] {
            let clause = track_order_by(sort);
            assert!(
                clause.ends_with("t.id ASC"),
                "{clause} has no stable tiebreak"
            );
            assert!(!clause.contains(';'));
        }
        assert!(track_order_by(TrackSort::TitleDesc).starts_with("t.sort_title DESC"));
        assert!(track_order_by(TrackSort::YearDesc).starts_with("t.year DESC"));
        assert!(track_order_by(TrackSort::ArtistAsc).starts_with("ar.sort_name ASC"));
    }
}
