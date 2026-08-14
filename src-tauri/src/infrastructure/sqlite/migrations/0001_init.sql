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

-- Preset smart playlists (CONTRACTS §1.4): rules are data, seeded here so no
-- query path ever hardcodes a preset. `sort` holds the SmartSort token and
-- `rules_json` the serialized Vec<SmartRule>.
INSERT INTO smart_playlists (name, rules_json, match_all, sort, limit_n, created_at, updated_at)
SELECT
  seed.name,
  seed.rules_json,
  seed.match_all,
  seed.sort,
  seed.limit_n,
  CAST(strftime('%s', 'now') AS INTEGER) * 1000,
  CAST(strftime('%s', 'now') AS INTEGER) * 1000
FROM (
  SELECT 1 AS ord, 'Recently Added' AS name,
         '[{"kind":"ADDED_WITHIN_DAYS","days":30}]' AS rules_json,
         1 AS match_all, 'DATE_ADDED_DESC' AS sort, 100 AS limit_n
  UNION ALL SELECT 2, 'Not Played',
         '[{"kind":"NEVER_PLAYED"}]',
         1, 'TITLE_ASC', NULL
  UNION ALL SELECT 3, 'Forgotten',
         '[{"kind":"LAST_PLAYED_BEFORE_DAYS","days":60}]',
         1, 'LAST_PLAYED_DESC', 100
  UNION ALL SELECT 4, 'Favorites',
         '[{"kind":"FAVORITE","value":true}]',
         1, 'DATE_ADDED_DESC', NULL
  UNION ALL SELECT 5, 'Most Played',
         '[{"kind":"PLAY_COUNT","op":"GTE","value":1}]',
         1, 'PLAY_COUNT_DESC', 100
  UNION ALL SELECT 6, '2020s',
         '[{"kind":"YEAR_BETWEEN","from":2020,"to":2029}]',
         1, 'YEAR_DESC', NULL
  UNION ALL SELECT 7, 'High Quality',
         '[{"kind":"BITRATE_AT_LEAST","value":320000}]',
         1, 'TITLE_ASC', NULL
) AS seed
ORDER BY seed.ord;
