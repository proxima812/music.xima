CREATE TABLE hidden_tracks (
  track_id INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
  uri TEXT NOT NULL UNIQUE,
  hidden_at INTEGER NOT NULL
);

CREATE TABLE pending_deletions (
  track_id INTEGER PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
  uri TEXT NOT NULL UNIQUE,
  requested_at INTEGER NOT NULL,
  file_deleted INTEGER NOT NULL DEFAULT 0 CHECK (file_deleted IN (0, 1))
);

CREATE INDEX idx_hidden_tracks_hidden_at ON hidden_tracks(hidden_at DESC);
CREATE INDEX idx_pending_deletions_file_deleted ON pending_deletions(file_deleted, requested_at);
