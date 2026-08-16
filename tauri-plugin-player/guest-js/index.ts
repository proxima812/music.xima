/**
 * TS-биндинги плагина `tauri-plugin-player` (CONTRACTS §7.4).
 *
 * Приложение их напрямую НЕ вызывает — фронтенд ходит в команды ядра из §5.
 * Здесь живут типы моста Rust↔Kotlin и обёртки для ручной отладки плеера.
 * Rust `Option<T>` → `T | null`, `i64`/`f32` → `number`.
 */

import { invoke } from '@tauri-apps/api/core'

const PREFIX = 'plugin:player|'

// §1.5 / §7.1 — состояние воспроизведения

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

// §7.1 — очередь

export type QueueItem = {
  trackId: number
  uri: string
  title: string
  artist: string | null
  album: string | null
  durationMs: number
  /** `file://` или `content://` на обложку в кэше приложения. */
  artworkUri: string | null
}

export type SetQueueRequest = {
  items: QueueItem[]
  startIndex: number
  autoplay: boolean
}

// §3 / §7.1 — результат нативного сканера

export type ScannedTrack = {
  uri: string
  title: string
  artist: string | null
  album: string | null
  albumArtist: string | null
  durationMs: number
  trackNumber: number | null
  discNumber: number | null
  year: number | null
  genre: string | null
  bitrate: number | null
  sampleRate: number | null
  size: number
  mimeType: string | null
  folder: string | null
  dateAdded: number
  lastModified: number
  coverKey: string | null
}

export type ScanBatch = {
  tracks: ScannedTrack[]
  complete: boolean
  /** Не `null` → сканирование не закончено, вызвать ещё раз с этим курсором. */
  nextCursor: string | null
}

// Воспроизведение

export const getState = (): Promise<PlaybackState> => invoke<PlaybackState>(`${PREFIX}get_state`)

export const getQueueIds = (): Promise<number[]> =>
  invoke<{ trackIds: number[] }>(`${PREFIX}get_queue_ids`).then((response) => response.trackIds)

export const setQueue = (req: SetQueueRequest): Promise<void> =>
  invoke<void>(`${PREFIX}set_queue`, { req })

export const play = (): Promise<void> => invoke<void>(`${PREFIX}play`)

export const pause = (): Promise<void> => invoke<void>(`${PREFIX}pause`)

export const toggle = (): Promise<void> => invoke<void>(`${PREFIX}toggle`)

export const stop = (): Promise<void> => invoke<void>(`${PREFIX}stop`)

export const next = (): Promise<void> => invoke<void>(`${PREFIX}next`)

export const previous = (): Promise<void> => invoke<void>(`${PREFIX}previous`)

export const seek = (positionMs: number): Promise<void> =>
  invoke<void>(`${PREFIX}seek`, { positionMs })

export const skipTo = (index: number): Promise<void> => invoke<void>(`${PREFIX}skip_to`, { index })

export const setShuffle = (enabled: boolean): Promise<void> =>
  invoke<void>(`${PREFIX}set_shuffle`, { enabled })

export const setRepeat = (mode: RepeatMode): Promise<void> =>
  invoke<void>(`${PREFIX}set_repeat`, { mode })

export const setVolume = (volume: number): Promise<void> =>
  invoke<void>(`${PREFIX}set_volume`, { volume })

export const setSpeed = (speed: number): Promise<void> =>
  invoke<void>(`${PREFIX}set_speed`, { speed })

export const setCrossfade = (durationMs: number): Promise<void> =>
  invoke<void>(`${PREFIX}set_crossfade`, { durationMs })

// Очередь

export const addNext = (items: QueueItem[]): Promise<void> =>
  invoke<void>(`${PREFIX}add_next`, { items })

export const addToQueue = (items: QueueItem[]): Promise<void> =>
  invoke<void>(`${PREFIX}add_to_queue`, { items })

export const removeQueueItem = (index: number): Promise<void> =>
  invoke<void>(`${PREFIX}remove_queue_item`, { index })

export const moveQueueItem = (from: number, to: number): Promise<void> =>
  invoke<void>(`${PREFIX}move_queue_item`, { from, to })

export const clearQueue = (): Promise<void> => invoke<void>(`${PREFIX}clear_queue`)

// Библиотека и доступ к файлам

export const scanMediaStore = (since: number | null = null): Promise<ScanBatch> =>
  invoke<ScanBatch>(`${PREFIX}scan_media_store`, { since })

export const scanTree = (treeUri: string, since: number | null = null): Promise<ScanBatch> =>
  invoke<ScanBatch>(`${PREFIX}scan_tree`, { treeUri, since })

/** SAF-пикер папки; `null`, если пользователь отменил выбор. */
export const pickFolder = (): Promise<string | null> => invoke<string | null>(`${PREFIX}pick_folder`)

export const persistedRoots = (): Promise<string[]> => invoke<string[]>(`${PREFIX}persisted_roots`)

export const releaseRoot = (treeUri: string): Promise<void> =>
  invoke<void>(`${PREFIX}release_root`, { treeUri })

/** `file://` на извлечённую обложку; `null`, если её нет в файле. */
export const extractArtwork = (uri: string): Promise<string | null> =>
  invoke<string | null>(`${PREFIX}extract_artwork`, { uri })
