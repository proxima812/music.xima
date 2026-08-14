import type { RepeatMode, TrackSort } from '../ipc/types'

/** Настройки приложения (CONTRACTS.md §9). Ничего сверх этого списка в Store не кладём. */

export type LastQueue = {
  trackIds: number[]
  index: number
  positionMs: number
}

export type AppSettings = {
  /** 0..1 */
  volume: number
  repeat: RepeatMode
  shuffle: boolean
  librarySort: TrackSort
  rememberQueue: boolean
  lastQueue: LastQueue | null
  scanRoots: string[]
  crossfadeMs: number
}

/** Верхняя граница кроссфейда — Media3 держит стык треков, дальше смысла нет. */
export const MAX_CROSSFADE_MS = 12_000

export const DEFAULT_SETTINGS: AppSettings = {
  volume: 1,
  repeat: 'OFF',
  shuffle: false,
  librarySort: 'TITLE_ASC',
  rememberQueue: true,
  lastQueue: null,
  scanRoots: [],
  crossfadeMs: 0,
}

export const SETTINGS_KEYS = [
  'volume',
  'repeat',
  'shuffle',
  'librarySort',
  'rememberQueue',
  'lastQueue',
  'scanRoots',
  'crossfadeMs',
] as const satisfies readonly (keyof AppSettings)[]

export function cloneSettings(source: AppSettings): AppSettings {
  return {
    ...source,
    lastQueue:
      source.lastQueue === null
        ? null
        : { ...source.lastQueue, trackIds: [...source.lastQueue.trackIds] },
    scanRoots: [...source.scanRoots],
  }
}
