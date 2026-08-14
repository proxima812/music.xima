/**
 * Зеркало Rust-типов из CONTRACTS.md (§1, §2, §5).
 * Только type/interface. Rust `Option<T>` → `T | null`, `i64`/`f32` → `number`.
 * Расхождение с Rust — баг: правится сначала CONTRACTS.md, потом обе стороны.
 */

// §2 — ошибки IPC

export type IpcErrorCode =
  | 'NOT_FOUND'
  | 'INVALID_INPUT'
  | 'DATABASE'
  | 'PLAYER'
  | 'SCAN'
  | 'IO'
  | 'INTERNAL'

export type IpcError = {
  code: IpcErrorCode
  message: string
}

// §1.1 — трек

export type Track = {
  id: number
  /** content:// URI (MediaStore или SAF). Никогда не абсолютный путь. */
  uri: string
  title: string
  artistId: number | null
  artistName: string | null
  albumId: number | null
  albumTitle: string | null
  durationMs: number
  trackNumber: number | null
  discNumber: number | null
  year: number | null
  genre: string | null
  bitrate: number | null
  sampleRate: number | null
  size: number
  format: string | null
  /** Ключ обложки в кэше приложения; резолвится командой `artwork_uri`. */
  coverKey: string | null
  folder: string | null
  dateAdded: number
  lastModified: number
  isFavorite: boolean
  playCount: number
  lastPlayedAt: number | null
}

// §1.2 — альбом, артист, жанр, папка

export type Album = {
  id: number
  title: string
  artistId: number | null
  artistName: string | null
  year: number | null
  coverKey: string | null
  trackCount: number
  durationMs: number
}

export type Artist = {
  id: number
  name: string
  albumCount: number
  trackCount: number
  coverKey: string | null
}

export type Genre = {
  name: string
  trackCount: number
}

export type Folder = {
  /** Display-путь, напр. "Music/MyMusic/Rock". */
  path: string
  name: string
  trackCount: number
}

// §1.3 — плейлист

export type Playlist = {
  id: number
  name: string
  trackCount: number
  durationMs: number
  coverKey: string | null
  createdAt: number
  updatedAt: number
}

// §1.4 — smart playlists

export type NumOp = 'EQ' | 'NE' | 'LT' | 'LTE' | 'GT' | 'GTE'

export type SmartRule =
  | { kind: 'PLAY_COUNT'; op: NumOp; value: number }
  | { kind: 'NEVER_PLAYED' }
  | { kind: 'LAST_PLAYED_BEFORE_DAYS'; days: number }
  | { kind: 'ADDED_WITHIN_DAYS'; days: number }
  | { kind: 'FAVORITE'; value: boolean }
  | { kind: 'YEAR_BETWEEN'; from: number; to: number }
  | { kind: 'BITRATE_AT_LEAST'; value: number }
  | { kind: 'DURATION_BETWEEN_MS'; from: number; to: number }
  | { kind: 'GENRE_IS'; value: string }
  | { kind: 'ARTIST_IS'; artistId: number }
  | { kind: 'TITLE_CONTAINS'; value: string }

export type SmartRuleKind = SmartRule['kind']

export type SmartSort =
  | 'TITLE_ASC'
  | 'DATE_ADDED_DESC'
  | 'PLAY_COUNT_DESC'
  | 'LAST_PLAYED_DESC'
  | 'YEAR_DESC'
  | 'RANDOM'

export type SmartPlaylist = {
  id: number
  name: string
  rules: SmartRule[]
  /** true = AND между правилами, false = OR. */
  matchAll: boolean
  sort: SmartSort
  limit: number | null
  createdAt: number
  updatedAt: number
}

/** §5 playlist.rs — черновик smart-плейлиста (create/update/preview). */
export type SmartPlaylistDraft = {
  name: string
  rules: SmartRule[]
  matchAll: boolean
  sort: SmartSort
  limit: number | null
}

// §1.5 — состояние воспроизведения

export type PlaybackStatus = 'IDLE' | 'BUFFERING' | 'PLAYING' | 'PAUSED' | 'ENDED'

export type RepeatMode = 'OFF' | 'ALL' | 'ONE'

export type PlaybackState = {
  status: PlaybackStatus
  trackId: number | null
  positionMs: number
  durationMs: number
  queueIndex: number | null
  queueLength: number
  shuffle: boolean
  repeat: RepeatMode
  volume: number
  speed: number
}

// §1.6 — запросы и постраничность

export type TrackSort =
  | 'TITLE_ASC'
  | 'TITLE_DESC'
  | 'ARTIST_ASC'
  | 'ALBUM_ASC'
  | 'DATE_ADDED_DESC'
  | 'DATE_ADDED_ASC'
  | 'DURATION_ASC'
  | 'DURATION_DESC'
  | 'PLAY_COUNT_DESC'
  | 'LAST_PLAYED_DESC'
  | 'YEAR_DESC'

export type TrackQuery = {
  sort: TrackSort
  offset: number
  limit: number
  artistId: number | null
  albumId: number | null
  genre: string | null
  folder: string | null
  favoritesOnly: boolean
}

export type Page<T> = {
  items: T[]
  total: number
  offset: number
  limit: number
}

export type LibraryStats = {
  tracks: number
  albums: number
  artists: number
  playlists: number
  genres: number
  totalDurationMs: number
  totalSize: number
}

export type SearchResults = {
  tracks: Track[]
  albums: Album[]
  artists: Artist[]
  playlists: Playlist[]
}

export type StatsRange = 'TODAY' | 'WEEK' | 'MONTH' | 'YEAR' | 'ALL_TIME'

export type RankedTrack = {
  track: Track
  plays: number
  listeningTimeMs: number
}

// §5 library.rs — сканирование

export type ScanMode = 'FULL' | 'INCREMENTAL'

export type ScanResult = {
  added: number
  updated: number
  removed: number
  durationMs: number
}

export type ScanStatus = {
  running: boolean
  scanned: number
  total: number
  phase: string
}
