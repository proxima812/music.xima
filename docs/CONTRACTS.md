# CONTRACTS.md — канонические границы music.xima

Этот файл — **единственный источник истины** для всего, что пересекает границу слоя:
типы домена, трейты репозиториев, имена и сигнатуры Tauri-команд, события,
API нативного плагина, схема БД.

Правила:

- Любой агент/разработчик реализует **ровно** то, что здесь написано. Имена, регистр,
  типы, nullability — буквально.
- Нужно изменить контракт → сначала правится этот файл, потом код по обе стороны границы.
- Сериализация: **все** DTO помечены `#[serde(rename_all = "camelCase")]`.
  Rust `snake_case` ↔ TypeScript `camelCase`.
- Время — `i64`, Unix-миллисекунды UTC. Длительности — `i64` мс. Идентификаторы — `i64`.
- Enum-ы сериализуются как `SCREAMING_SNAKE_CASE` строки
  (`#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`), кроме помеченных отдельно.

---

## 1. Домен (`src-tauri/src/domain/`)

### 1.1 `track.rs`

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: i64,
    /// content:// URI (MediaStore или SAF). Никогда не абсолютный путь.
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
    /// Ключ обложки в кэше приложения; резолвится `artwork_uri`.
    pub cover_key: Option<String>,
    /// Родительская папка (для режима Folders), display-путь из SAF/MediaStore.
    pub folder: Option<String>,
    pub date_added: i64,
    pub last_modified: i64,
    pub is_favorite: bool,
    pub play_count: i64,
    pub last_played_at: Option<i64>,
}
```

### 1.2 `album.rs`, `artist.rs`

```rust
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

#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: i64,
    pub name: String,
    pub album_count: i64,
    pub track_count: i64,
    pub cover_key: Option<String>,
}

#[serde(rename_all = "camelCase")]
pub struct Genre {
    pub name: String,
    pub track_count: i64,
}

#[serde(rename_all = "camelCase")]
pub struct Folder {
    /// Display-путь, напр. "Music/MyMusic/Rock".
    pub path: String,
    pub name: String,
    pub track_count: i64,
}
```

### 1.3 `playlist.rs`

```rust
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
```

### 1.4 `smart.rs` — Smart Playlists

Правило — это **данные**, компилируемые в SQL. Отдельного метода на пресет быть не должно.

```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NumOp { Eq, Ne, Lt, Lte, Gt, Gte }

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SmartRule {
    #[serde(rename_all = "camelCase")]
    PlayCount { op: NumOp, value: i64 },
    NeverPlayed,
    #[serde(rename_all = "camelCase")]
    LastPlayedBeforeDays { days: i64 },
    #[serde(rename_all = "camelCase")]
    AddedWithinDays { days: i64 },
    #[serde(rename_all = "camelCase")]
    Favorite { value: bool },
    #[serde(rename_all = "camelCase")]
    YearBetween { from: i32, to: i32 },
    #[serde(rename_all = "camelCase")]
    BitrateAtLeast { value: i32 },
    #[serde(rename_all = "camelCase")]
    DurationBetweenMs { from: i64, to: i64 },
    #[serde(rename_all = "camelCase")]
    GenreIs { value: String },
    #[serde(rename_all = "camelCase")]
    ArtistIs { artist_id: i64 },
    #[serde(rename_all = "camelCase")]
    TitleContains { value: String },
}

#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SmartSort {
    TitleAsc, DateAddedDesc, PlayCountDesc, LastPlayedDesc, YearDesc, Random,
}

#[serde(rename_all = "camelCase")]
pub struct SmartPlaylist {
    pub id: i64,
    pub name: String,
    pub rules: Vec<SmartRule>,
    /// true = AND между правилами, false = OR.
    pub match_all: bool,
    pub sort: SmartSort,
    pub limit: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}
```

Пресеты («Недавно добавленные», «Ни разу не играли», «Забытые», «Избранное»,
«Часто слушаю», «2020-е», «Высокое качество») создаются как обычные `SmartPlaylist`
при первом запуске, сидом в миграции — **не** хардкодом в коде запросов.
Сид `0001` остаётся английским (менять применённую миграцию нельзя), русские имена
проставляет `0002_ru_smart_presets.sql`.

### 1.5 `queue.rs` / состояние воспроизведения

```rust
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlaybackStatus { Idle, Buffering, Playing, Paused, Ended }

#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RepeatMode { Off, All, One }

#[serde(rename_all = "camelCase")]
pub struct PlaybackState {
    pub status: PlaybackStatus,
    pub track_id: Option<i64>,
    pub position_ms: i64,
    pub duration_ms: i64,
    pub queue_index: Option<i32>,
    pub queue_length: i32,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub volume: f32,
    pub speed: f32,
}
```

**Источник истины по `PlaybackState` — нативный плеер.** Rust его не хранит и не
вычисляет, только пробрасывает.

### 1.6 Запросы и постраничность

```rust
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TrackSort {
    TitleAsc, TitleDesc, ArtistAsc, AlbumAsc,
    DateAddedDesc, DateAddedAsc, DurationAsc, DurationDesc,
    PlayCountDesc, LastPlayedDesc, YearDesc,
}

#[serde(rename_all = "camelCase")]
pub struct TrackQuery {
    pub sort: TrackSort,
    pub offset: i64,
    pub limit: i64,
    pub artist_id: Option<i64>,
    pub album_id: Option<i64>,
    pub genre: Option<String>,
    pub folder: Option<String>,
    pub favorites_only: bool,
}

#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub offset: i64,
    pub limit: i64,
}

#[serde(rename_all = "camelCase")]
pub struct LibraryStats {
    pub tracks: i64,
    pub albums: i64,
    pub artists: i64,
    pub playlists: i64,
    pub genres: i64,
    pub total_duration_ms: i64,
    pub total_size: i64,
}

#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
}

#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StatsRange { Today, Week, Month, Year, AllTime }

#[serde(rename_all = "camelCase")]
pub struct RankedTrack { pub track: Track, pub plays: i64, pub listening_time_ms: i64 }
```

---

## 2. Ошибки (`src-tauri/src/error.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("not found: {0}")]        NotFound(String),
    #[error("invalid input: {0}")]    InvalidInput(String),
    #[error("database error: {0}")]   Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]  Migration(#[from] sqlx::migrate::MigrateError),
    #[error("player error: {0}")]     Player(String),
    #[error("scan error: {0}")]       Scan(String),
    #[error("io error: {0}")]         Io(#[from] std::io::Error),
    #[error("internal: {0}")]         Internal(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
```

Наружу в IPC ошибка отдаётся сериализуемым видом:

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IpcError { pub code: &'static str, pub message: String }
```

`code` ∈ `NOT_FOUND | INVALID_INPUT | DATABASE | PLAYER | SCAN | IO | INTERNAL`.
`impl serde::Serialize for CoreError` конвертирует в `IpcError`.

---

## 3. Репозитории (`src-tauri/src/infrastructure/repositories/`)

Трейты объявляются здесь, реализуются в `infrastructure/sqlite/`.
Все `async`, все `#[async_trait::async_trait]`, все возвращают `CoreResult<T>`.

```rust
#[async_trait]
pub trait TrackRepository: Send + Sync {
    async fn get(&self, id: i64) -> CoreResult<Track>;
    async fn get_many(&self, ids: &[i64]) -> CoreResult<Vec<Track>>;
    async fn query(&self, q: &TrackQuery) -> CoreResult<Page<Track>>;
    async fn recently_added(&self, limit: i64) -> CoreResult<Vec<Track>>;
    async fn upsert_many(&self, tracks: &[ScannedTrack]) -> CoreResult<u64>;
    async fn delete_missing(&self, keep_uris: &[String]) -> CoreResult<u64>;
    async fn stats(&self) -> CoreResult<LibraryStats>;
}

#[async_trait]
pub trait AlbumRepository: Send + Sync {
    async fn get(&self, id: i64) -> CoreResult<Album>;
    async fn query(&self, offset: i64, limit: i64) -> CoreResult<Page<Album>>;
    async fn by_artist(&self, artist_id: i64) -> CoreResult<Vec<Album>>;
}

#[async_trait]
pub trait ArtistRepository: Send + Sync {
    async fn get(&self, id: i64) -> CoreResult<Artist>;
    async fn query(&self, offset: i64, limit: i64) -> CoreResult<Page<Artist>>;
}

#[async_trait]
pub trait TaxonomyRepository: Send + Sync {
    async fn genres(&self) -> CoreResult<Vec<Genre>>;
    async fn folders(&self, parent: Option<&str>) -> CoreResult<Vec<Folder>>;
}

#[async_trait]
pub trait PlaylistRepository: Send + Sync {
    async fn list(&self) -> CoreResult<Vec<Playlist>>;
    async fn get(&self, id: i64) -> CoreResult<Playlist>;
    async fn create(&self, name: &str) -> CoreResult<Playlist>;
    async fn rename(&self, id: i64, name: &str) -> CoreResult<()>;
    async fn delete(&self, id: i64) -> CoreResult<()>;
    async fn tracks(&self, id: i64) -> CoreResult<Vec<Track>>;
    async fn add_tracks(&self, id: i64, track_ids: &[i64]) -> CoreResult<()>;
    async fn remove_at(&self, id: i64, position: i64) -> CoreResult<()>;
    async fn reorder(&self, id: i64, from: i64, to: i64) -> CoreResult<()>;
}

#[async_trait]
pub trait SmartPlaylistRepository: Send + Sync {
    async fn list(&self) -> CoreResult<Vec<SmartPlaylist>>;
    async fn get(&self, id: i64) -> CoreResult<SmartPlaylist>;
    async fn create(&self, draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylist>;
    async fn update(&self, id: i64, draft: &SmartPlaylistDraft) -> CoreResult<SmartPlaylist>;
    async fn delete(&self, id: i64) -> CoreResult<()>;
    /// Компилирует правила в SQL и возвращает треки.
    async fn resolve(&self, rules: &[SmartRule], match_all: bool,
                     sort: SmartSort, limit: Option<i64>) -> CoreResult<Vec<Track>>;
}

#[async_trait]
pub trait FavoriteRepository: Send + Sync {
    async fn toggle(&self, track_id: i64) -> CoreResult<bool>;
    async fn list(&self) -> CoreResult<Vec<Track>>;
}

#[async_trait]
pub trait HistoryRepository: Send + Sync {
    async fn record(&self, track_id: i64, played_at: i64, duration_played_ms: i64) -> CoreResult<()>;
    async fn recent(&self, limit: i64) -> CoreResult<Vec<Track>>;
}

#[async_trait]
pub trait StatisticsRepository: Send + Sync {
    async fn top_tracks(&self, range: StatsRange, limit: i64) -> CoreResult<Vec<RankedTrack>>;
    async fn never_played(&self, limit: i64) -> CoreResult<Vec<Track>>;
    async fn forgotten(&self, days: i64, limit: i64) -> CoreResult<Vec<Track>>;
}

#[async_trait]
pub trait SearchRepository: Send + Sync {
    async fn all(&self, q: &str, limit: i64) -> CoreResult<SearchResults>;
    async fn tracks(&self, q: &str, limit: i64) -> CoreResult<Vec<Track>>;
}
```

`ScannedTrack` — то, что отдаёт нативный сканер (см. §6.3), объявляется в
`domain/scan.rs`:

```rust
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
```

---

## 4. Схема БД (`src-tauri/src/infrastructure/sqlite/migrations/`)

Миграции — файлы `0001_init.sql`, `0002_....sql`, применяются `sqlx::migrate!`.
Применённую миграцию менять нельзя.

`0001_init.sql`:

```sql
PRAGMA foreign_keys = ON;

CREATE TABLE artists (
  id    INTEGER PRIMARY KEY AUTOINCREMENT,
  name  TEXT NOT NULL,
  sort_name TEXT NOT NULL,
  UNIQUE(sort_name)
);

CREATE TABLE albums (
  id        INTEGER PRIMARY KEY AUTOINCREMENT,
  artist_id INTEGER REFERENCES artists(id) ON DELETE SET NULL,
  title     TEXT NOT NULL,
  sort_title TEXT NOT NULL,
  year      INTEGER,
  cover_key TEXT,
  UNIQUE(sort_title, artist_id)
);

CREATE TABLE tracks (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  uri           TEXT NOT NULL UNIQUE,
  title         TEXT NOT NULL,
  sort_title    TEXT NOT NULL,
  artist_id     INTEGER REFERENCES artists(id) ON DELETE SET NULL,
  album_id      INTEGER REFERENCES albums(id)  ON DELETE SET NULL,
  duration_ms   INTEGER NOT NULL DEFAULT 0,
  track_number  INTEGER,
  disc_number   INTEGER,
  year          INTEGER,
  genre         TEXT,
  bitrate       INTEGER,
  sample_rate   INTEGER,
  size          INTEGER NOT NULL DEFAULT 0,
  format        TEXT,
  cover_key     TEXT,
  folder        TEXT,
  date_added    INTEGER NOT NULL,
  last_modified INTEGER NOT NULL
);

CREATE INDEX idx_tracks_artist      ON tracks(artist_id);
CREATE INDEX idx_tracks_album       ON tracks(album_id, disc_number, track_number);
CREATE INDEX idx_tracks_date_added  ON tracks(date_added DESC);
CREATE INDEX idx_tracks_sort_title  ON tracks(sort_title);
CREATE INDEX idx_tracks_genre       ON tracks(genre);
CREATE INDEX idx_tracks_folder      ON tracks(folder);

CREATE TABLE playlists (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE playlist_tracks (
  playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
  track_id    INTEGER NOT NULL REFERENCES tracks(id)    ON DELETE CASCADE,
  position    INTEGER NOT NULL,
  PRIMARY KEY (playlist_id, position)
);
CREATE INDEX idx_playlist_tracks_track ON playlist_tracks(track_id);

CREATE TABLE smart_playlists (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL,
  rules_json TEXT NOT NULL,
  match_all  INTEGER NOT NULL DEFAULT 1,
  sort       TEXT NOT NULL,
  limit_n    INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE TABLE favorites (
  track_id   INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
  created_at INTEGER NOT NULL
);

CREATE TABLE history (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  track_id           INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
  played_at          INTEGER NOT NULL,
  duration_played_ms INTEGER NOT NULL
);
CREATE INDEX idx_history_played_at ON history(played_at DESC);
CREATE INDEX idx_history_track     ON history(track_id);

CREATE TABLE play_counts (
  track_id       INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
  count          INTEGER NOT NULL DEFAULT 0,
  last_played_at INTEGER
);
CREATE INDEX idx_play_counts_count ON play_counts(count DESC);

-- Полнотекстовый поиск.
CREATE VIRTUAL TABLE tracks_fts USING fts5(
  title, artist, album, genre,
  content='',
  tokenize='unicode61 remove_diacritics 2'
);
```

FTS5 наполняется **явно** из репозитория при upsert/delete треков
(`content=''` — внешнего контента нет, триггеры не используются, потому что
artist/album лежат в других таблицах). Строка FTS адресуется через `rowid = tracks.id`.

---

## 5. Tauri-команды (`src-tauri/src/commands/`)

Имя команды в Rust = `snake_case`, из фронта вызывается **той же строкой**.
Все команды `async`, возвращают `Result<T, CoreError>`.
Аргументы фронт передаёт объектом в `camelCase`.

### library.rs

| Команда | Аргументы | Ответ |
| --- | --- | --- |
| `library_stats` | — | `LibraryStats` |
| `library_tracks` | `query: TrackQuery` | `Page<Track>` |
| `library_track` | `id: i64` | `Track` |
| `library_tracks_by_ids` | `ids: Vec<i64>` | `Vec<Track>` |
| `library_recently_added` | `limit: i64` | `Vec<Track>` |
| `library_albums` | `offset: i64, limit: i64` | `Page<Album>` |
| `library_album` | `id: i64` | `Album` |
| `library_album_tracks` | `albumId: i64` | `Vec<Track>` |
| `library_artists` | `offset: i64, limit: i64` | `Page<Artist>` |
| `library_artist` | `id: i64` | `Artist` |
| `library_artist_albums` | `artistId: i64` | `Vec<Album>` |
| `library_artist_tracks` | `artistId: i64` | `Vec<Track>` |
| `library_genres` | — | `Vec<Genre>` |
| `library_folders` | `parent: Option<String>` | `Vec<Folder>` |
| `library_scan` | `roots: Vec<String>, mode: ScanMode` | `ScanResult` |
| `library_scan_status` | — | `ScanStatus` |
| `library_pick_folder` | — | `Option<String>` (SAF tree URI) |
| `library_roots` | — | `Vec<String>` |
| `library_remove_root` | `uri: String` | `()` |
| `artwork_uri` | `coverKey: String` | `Option<String>` |

```rust
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScanMode { Full, Incremental }

#[serde(rename_all = "camelCase")]
pub struct ScanResult { pub added: i64, pub updated: i64, pub removed: i64, pub duration_ms: i64 }

#[serde(rename_all = "camelCase")]
pub struct ScanStatus { pub running: bool, pub scanned: i64, pub total: i64, pub phase: String }
```

### search.rs

| Команда | Аргументы | Ответ |
| --- | --- | --- |
| `search_all` | `q: String, limit: i64` | `SearchResults` |
| `search_tracks` | `q: String, limit: i64` | `Vec<Track>` |

### playlist.rs

| Команда | Аргументы | Ответ |
| --- | --- | --- |
| `playlist_list` | — | `Vec<Playlist>` |
| `playlist_get` | `id: i64` | `Playlist` |
| `playlist_create` | `name: String` | `Playlist` |
| `playlist_rename` | `id: i64, name: String` | `()` |
| `playlist_delete` | `id: i64` | `()` |
| `playlist_tracks` | `id: i64` | `Vec<Track>` |
| `playlist_add_tracks` | `id: i64, trackIds: Vec<i64>` | `()` |
| `playlist_remove_at` | `id: i64, position: i64` | `()` |
| `playlist_reorder` | `id: i64, from: i64, to: i64` | `()` |
| `smart_playlist_list` | — | `Vec<SmartPlaylist>` |
| `smart_playlist_get` | `id: i64` | `SmartPlaylist` |
| `smart_playlist_create` | `draft: SmartPlaylistDraft` | `SmartPlaylist` |
| `smart_playlist_update` | `id: i64, draft: SmartPlaylistDraft` | `SmartPlaylist` |
| `smart_playlist_delete` | `id: i64` | `()` |
| `smart_playlist_resolve` | `id: i64` | `Vec<Track>` |
| `smart_playlist_preview` | `draft: SmartPlaylistDraft` | `Vec<Track>` |

```rust
#[serde(rename_all = "camelCase")]
pub struct SmartPlaylistDraft {
    pub name: String,
    pub rules: Vec<SmartRule>,
    pub match_all: bool,
    pub sort: SmartSort,
    pub limit: Option<i64>,
}
```

### favorites.rs / history / statistics.rs

| Команда | Аргументы | Ответ |
| --- | --- | --- |
| `favorite_toggle` | `trackId: i64` | `bool` (новое состояние) |
| `favorite_list` | — | `Vec<Track>` |
| `history_record` | `trackId: i64, playedAt: i64, durationPlayedMs: i64` | `()` |
| `history_recent` | `limit: i64` | `Vec<Track>` |
| `stats_top_tracks` | `range: StatsRange, limit: i64` | `Vec<RankedTrack>` |
| `stats_never_played` | `limit: i64` | `Vec<Track>` |
| `stats_forgotten` | `days: i64, limit: i64` | `Vec<Track>` |

> `history_record` вызывает **Rust**, получив событие завершения трека от нативного
> слоя, а не фронтенд. Команда существует для ручных сценариев и тестов.

### player.rs — прокси в нативный плагин

| Команда | Аргументы | Ответ |
| --- | --- | --- |
| `player_state` | — | `PlaybackState` |
| `player_set_queue` | `trackIds: Vec<i64>, startIndex: i32, autoplay: bool` | `()` |
| `player_queue` | — | `Vec<Track>` |
| `player_play` | — | `()` |
| `player_pause` | — | `()` |
| `player_toggle` | — | `()` |
| `player_stop` | — | `()` |
| `player_next` | — | `()` |
| `player_previous` | — | `()` |
| `player_seek` | `positionMs: i64` | `()` |
| `player_skip_to` | `index: i32` | `()` |
| `player_set_shuffle` | `enabled: bool` | `()` |
| `player_set_repeat` | `mode: RepeatMode` | `()` |
| `player_set_volume` | `volume: f32` | `()` |
| `player_set_speed` | `speed: f32` | `()` |
| `player_set_crossfade` | `durationMs: i64` | `()` |
| `player_add_next` | `trackIds: Vec<i64>` | `()` |
| `player_add_to_queue` | `trackIds: Vec<i64>` | `()` |
| `player_remove_queue_item` | `index: i32` | `()` |
| `player_move_queue_item` | `from: i32, to: i32` | `()` |
| `player_clear_queue` | — | `()` |

---

## 6. События (Tauri events, `emit` из Rust)

| Событие | Payload | Когда |
| --- | --- | --- |
| `player:state` | `PlaybackState` | любое изменение состояния плеера |
| `player:track-changed` | `{ trackId: i64 \| null, index: i32 }` | смена текущего элемента |
| `player:queue-changed` | `{ trackIds: Vec<i64> }` | очередь изменилась |
| `player:completed` | `{ trackId: i64, durationPlayedMs: i64 }` | трек доигран/сменён — Rust пишет history |
| `player:error` | `{ code: String, message: String }` | ошибка воспроизведения |
| `library:scan-progress` | `ScanStatus` | прогресс сканирования |
| `library:changed` | `{ reason: String }` | библиотека изменилась, фронту нужно перечитать |

Фронтенд подписывается через `@tauri-apps/api/event`. Имена — константы в
`src/shared/ipc/events.ts`, руками строки не писать.

---

## 7. Нативный плагин `tauri-plugin-player`

### 7.1 Rust API (`tauri-plugin-player/src/`)

```rust
pub trait PlayerExt<R: tauri::Runtime> {
    fn player(&self) -> &Player<R>;
}

impl<R: Runtime> Player<R> {
    pub fn get_state(&self) -> crate::Result<PlaybackState>;
    pub fn set_queue(&self, req: SetQueueRequest) -> crate::Result<()>;
    pub fn play(&self) -> crate::Result<()>;
    pub fn pause(&self) -> crate::Result<()>;
    pub fn toggle(&self) -> crate::Result<()>;
    pub fn stop(&self) -> crate::Result<()>;
    pub fn next(&self) -> crate::Result<()>;
    pub fn previous(&self) -> crate::Result<()>;
    pub fn seek(&self, position_ms: i64) -> crate::Result<()>;
    pub fn skip_to(&self, index: i32) -> crate::Result<()>;
    pub fn set_shuffle(&self, enabled: bool) -> crate::Result<()>;
    pub fn set_repeat(&self, mode: RepeatMode) -> crate::Result<()>;
    pub fn set_volume(&self, volume: f32) -> crate::Result<()>;
    pub fn set_speed(&self, speed: f32) -> crate::Result<()>;
    pub fn set_crossfade(&self, duration_ms: i64) -> crate::Result<()>;
    pub fn add_next(&self, items: Vec<QueueItem>) -> crate::Result<()>;
    pub fn add_to_queue(&self, items: Vec<QueueItem>) -> crate::Result<()>;
    pub fn remove_queue_item(&self, index: i32) -> crate::Result<()>;
    pub fn move_queue_item(&self, from: i32, to: i32) -> crate::Result<()>;
    pub fn clear_queue(&self) -> crate::Result<()>;

    // Библиотека / доступ к файлам
    pub fn scan_media_store(&self, since: Option<i64>) -> crate::Result<ScanBatch>;
    pub fn scan_tree(&self, tree_uri: String, since: Option<i64>) -> crate::Result<ScanBatch>;
    pub fn pick_folder(&self) -> crate::Result<Option<String>>;
    pub fn persisted_roots(&self) -> crate::Result<Vec<String>>;
    pub fn release_root(&self, tree_uri: String) -> crate::Result<()>;
    pub fn extract_artwork(&self, uri: String) -> crate::Result<Option<String>>;
}
```

`QueueItem` — то, что нужно нативному плееру, чтобы играть и рисовать нотификацию:

```rust
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub track_id: i64,
    pub uri: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: i64,
    /// file:// или content:// на обложку в кэше приложения.
    pub artwork_uri: Option<String>,
}

#[serde(rename_all = "camelCase")]
pub struct SetQueueRequest { pub items: Vec<QueueItem>, pub start_index: i32, pub autoplay: bool }

#[serde(rename_all = "camelCase")]
pub struct ScanBatch { pub tracks: Vec<ScannedTrack>, pub complete: bool, pub next_cursor: Option<String> }
```

### 7.2 Kotlin `@Command`-методы (`PlayerPlugin.kt`)

Имена **camelCase**, ровно эти:

```
getState, setQueue, play, pause, toggle, stop, next, previous,
seek, skipTo, setShuffle, setRepeat, setVolume, setSpeed, setCrossfade,
addNext, addToQueue, removeQueueItem, moveQueueItem, clearQueue,
scanMediaStore, scanTree, pickFolder, persistedRoots, releaseRoot, extractArtwork
```

Плагин шлёт наверх события через `trigger(event, data)`:

```
"state"      -> PlaybackState
"trackChanged" -> { trackId, index }
"queueChanged" -> { trackIds: [Long] }
"completed"  -> { trackId, durationPlayedMs }
"error"      -> { code, message }
"scanProgress" -> { scanned, total, phase }
```

Rust-сторона слушает их и ре-эмитит как Tauri-события из §6.

### 7.3 Desktop-заглушка

На не-Android целях плагин компилируется в no-op реализацию, возвращающую
`PlaybackState { status: IDLE, .. }` и `Err(PlayerError::Unsupported)` на
командах воспроизведения. Это нужно только чтобы `cargo check`/юнит-тесты
Rust-ядра работали на macOS/Linux.

### 7.4 TS-биндинги (`tauri-plugin-player/guest-js/index.ts`)

Экспортирует типы `PlaybackState`, `RepeatMode`, `PlaybackStatus`, `QueueItem`
и функции-обёртки над `invoke('plugin:player|<command>')`.
**Фронтенд приложения их напрямую не вызывает** — он ходит в команды ядра из §5.
Биндинги существуют для типов и для отладки.

---

## 8. TypeScript-зеркало (`src/shared/ipc/`)

- `types.ts` — типы 1:1 с §1 и §5. Только `type`/`interface`, никаких классов.
  Enum-ы — union-строки: `type RepeatMode = 'OFF' | 'ALL' | 'ONE'`.
- `commands.ts` — по одной типизированной функции на команду из §5:
  ```ts
  export const libraryTracks = (query: TrackQuery): Promise<Page<Track>> =>
    call('library_tracks', { query })
  ```
- `call.ts` — единственное место, где вызывается `invoke`; нормализует ошибку в
  `IpcError` и логирует.
- `events.ts` — константы имён событий + типизированный `listen`.

Никто, кроме `src/shared/ipc/`, не импортирует `@tauri-apps/api/core`.

---

## 9. Настройки (`@tauri-apps/plugin-store`)

Файл `settings.json`. Только это, ничего больше:

```ts
type AppSettings = {
  volume: number            // 0..1
  repeat: RepeatMode
  shuffle: boolean
  librarySort: TrackSort
  rememberQueue: boolean
  lastQueue: { trackIds: number[]; index: number; positionMs: number } | null
  scanRoots: string[]
  crossfadeMs: number       // 0..12000, плавный переход между треками
}
```

Дефолты — `src/shared/settings/defaults.ts`.

`repeat`, `shuffle`, `volume` и `crossfadeMs` — настройки, а не состояние плеера:
нативная сессия переживает не каждый перезапуск, поэтому на старте
`PlayerProvider` досылает их командами `player_set_*`.

Темы и размера сетки в настройках нет: приложение всегда тёмное (`class="dark"`
жёстко в `index.html`), сетка одна на всё приложение.
