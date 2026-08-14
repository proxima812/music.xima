import { call } from './call'
import type {
  Album,
  Artist,
  Folder,
  Genre,
  LibraryStats,
  Page,
  PlaybackState,
  Playlist,
  RankedTrack,
  RepeatMode,
  ScanMode,
  ScanResult,
  ScanStatus,
  SearchResults,
  SmartPlaylist,
  SmartPlaylistDraft,
  StatsRange,
  Track,
  TrackQuery,
} from './types'

/**
 * Типизированные обёртки над командами ядра (CONTRACTS.md §5).
 * Имя команды — строка из контракта, имя функции — её camelCase.
 */

// ─── library.rs ──────────────────────────────────────────────────────────────

export const libraryStats = (): Promise<LibraryStats> => call('library_stats')

export const libraryTracks = (query: TrackQuery): Promise<Page<Track>> =>
  call('library_tracks', { query })

export const libraryTrack = (id: number): Promise<Track> => call('library_track', { id })

export const libraryTracksByIds = (ids: number[]): Promise<Track[]> =>
  call('library_tracks_by_ids', { ids })

export const libraryRecentlyAdded = (limit: number): Promise<Track[]> =>
  call('library_recently_added', { limit })

export const libraryAlbums = (offset: number, limit: number): Promise<Page<Album>> =>
  call('library_albums', { offset, limit })

export const libraryAlbum = (id: number): Promise<Album> => call('library_album', { id })

export const libraryAlbumTracks = (albumId: number): Promise<Track[]> =>
  call('library_album_tracks', { albumId })

export const libraryArtists = (offset: number, limit: number): Promise<Page<Artist>> =>
  call('library_artists', { offset, limit })

export const libraryArtist = (id: number): Promise<Artist> => call('library_artist', { id })

export const libraryArtistAlbums = (artistId: number): Promise<Album[]> =>
  call('library_artist_albums', { artistId })

export const libraryArtistTracks = (artistId: number): Promise<Track[]> =>
  call('library_artist_tracks', { artistId })

export const libraryGenres = (): Promise<Genre[]> => call('library_genres')

export const libraryFolders = (parent: string | null = null): Promise<Folder[]> =>
  call('library_folders', { parent })

export const libraryScan = (roots: string[], mode: ScanMode): Promise<ScanResult> =>
  call('library_scan', { roots, mode })

export const libraryScanStatus = (): Promise<ScanStatus> => call('library_scan_status')

/** Открывает системный picker SAF, возвращает tree URI или null, если отменено. */
export const libraryPickFolder = (): Promise<string | null> => call('library_pick_folder')

export const libraryRoots = (): Promise<string[]> => call('library_roots')

export const libraryRemoveRoot = (uri: string): Promise<void> =>
  call('library_remove_root', { uri })

export const artworkUri = (coverKey: string): Promise<string | null> =>
  call('artwork_uri', { coverKey })

// ─── search.rs ───────────────────────────────────────────────────────────────

export const searchAll = (q: string, limit: number): Promise<SearchResults> =>
  call('search_all', { q, limit })

export const searchTracks = (q: string, limit: number): Promise<Track[]> =>
  call('search_tracks', { q, limit })

// ─── playlist.rs ─────────────────────────────────────────────────────────────

export const playlistList = (): Promise<Playlist[]> => call('playlist_list')

export const playlistGet = (id: number): Promise<Playlist> => call('playlist_get', { id })

export const playlistCreate = (name: string): Promise<Playlist> => call('playlist_create', { name })

export const playlistRename = (id: number, name: string): Promise<void> =>
  call('playlist_rename', { id, name })

export const playlistDelete = (id: number): Promise<void> => call('playlist_delete', { id })

export const playlistTracks = (id: number): Promise<Track[]> => call('playlist_tracks', { id })

export const playlistAddTracks = (id: number, trackIds: number[]): Promise<void> =>
  call('playlist_add_tracks', { id, trackIds })

export const playlistRemoveAt = (id: number, position: number): Promise<void> =>
  call('playlist_remove_at', { id, position })

export const playlistReorder = (id: number, from: number, to: number): Promise<void> =>
  call('playlist_reorder', { id, from, to })

export const smartPlaylistList = (): Promise<SmartPlaylist[]> => call('smart_playlist_list')

export const smartPlaylistGet = (id: number): Promise<SmartPlaylist> =>
  call('smart_playlist_get', { id })

export const smartPlaylistCreate = (draft: SmartPlaylistDraft): Promise<SmartPlaylist> =>
  call('smart_playlist_create', { draft })

export const smartPlaylistUpdate = (
  id: number,
  draft: SmartPlaylistDraft,
): Promise<SmartPlaylist> => call('smart_playlist_update', { id, draft })

export const smartPlaylistDelete = (id: number): Promise<void> =>
  call('smart_playlist_delete', { id })

export const smartPlaylistResolve = (id: number): Promise<Track[]> =>
  call('smart_playlist_resolve', { id })

export const smartPlaylistPreview = (draft: SmartPlaylistDraft): Promise<Track[]> =>
  call('smart_playlist_preview', { draft })

// ─── favorites.rs / history / statistics.rs ──────────────────────────────────

/** Возвращает новое состояние флага «избранное». */
export const favoriteToggle = (trackId: number): Promise<boolean> =>
  call('favorite_toggle', { trackId })

export const favoriteList = (): Promise<Track[]> => call('favorite_list')

export const historyRecord = (
  trackId: number,
  playedAt: number,
  durationPlayedMs: number,
): Promise<void> => call('history_record', { trackId, playedAt, durationPlayedMs })

export const historyRecent = (limit: number): Promise<Track[]> => call('history_recent', { limit })

export const statsTopTracks = (range: StatsRange, limit: number): Promise<RankedTrack[]> =>
  call('stats_top_tracks', { range, limit })

export const statsNeverPlayed = (limit: number): Promise<Track[]> =>
  call('stats_never_played', { limit })

export const statsForgotten = (days: number, limit: number): Promise<Track[]> =>
  call('stats_forgotten', { days, limit })

// ─── player.rs ───────────────────────────────────────────────────────────────

export const playerState = (): Promise<PlaybackState> => call('player_state')

export const playerSetQueue = (
  trackIds: number[],
  startIndex: number,
  autoplay: boolean,
): Promise<void> => call('player_set_queue', { trackIds, startIndex, autoplay })

export const playerQueue = (): Promise<Track[]> => call('player_queue')

export const playerPlay = (): Promise<void> => call('player_play')

export const playerPause = (): Promise<void> => call('player_pause')

export const playerToggle = (): Promise<void> => call('player_toggle')

export const playerStop = (): Promise<void> => call('player_stop')

export const playerNext = (): Promise<void> => call('player_next')

export const playerPrevious = (): Promise<void> => call('player_previous')

export const playerSeek = (positionMs: number): Promise<void> =>
  call('player_seek', { positionMs })

export const playerSkipTo = (index: number): Promise<void> => call('player_skip_to', { index })

export const playerSetShuffle = (enabled: boolean): Promise<void> =>
  call('player_set_shuffle', { enabled })

export const playerSetRepeat = (mode: RepeatMode): Promise<void> =>
  call('player_set_repeat', { mode })

export const playerSetVolume = (volume: number): Promise<void> =>
  call('player_set_volume', { volume })

export const playerSetSpeed = (speed: number): Promise<void> => call('player_set_speed', { speed })

export const playerAddNext = (trackIds: number[]): Promise<void> =>
  call('player_add_next', { trackIds })

export const playerAddToQueue = (trackIds: number[]): Promise<void> =>
  call('player_add_to_queue', { trackIds })

export const playerRemoveQueueItem = (index: number): Promise<void> =>
  call('player_remove_queue_item', { index })

export const playerMoveQueueItem = (from: number, to: number): Promise<void> =>
  call('player_move_queue_item', { from, to })

export const playerClearQueue = (): Promise<void> => call('player_clear_queue')
