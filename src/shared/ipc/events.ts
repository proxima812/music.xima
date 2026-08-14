import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type { PlaybackState, ScanStatus } from './types'

/**
 * События, которые Rust шлёт наверх (CONTRACTS.md §6).
 * Строки руками не писать — только через `AppEvent`.
 */

export const AppEvent = {
  PLAYER_STATE: 'player:state',
  PLAYER_TRACK_CHANGED: 'player:track-changed',
  PLAYER_QUEUE_CHANGED: 'player:queue-changed',
  PLAYER_COMPLETED: 'player:completed',
  PLAYER_ERROR: 'player:error',
  LIBRARY_SCAN_PROGRESS: 'library:scan-progress',
  LIBRARY_CHANGED: 'library:changed',
} as const

export type TrackChangedPayload = {
  trackId: number | null
  index: number
}

export type QueueChangedPayload = {
  trackIds: number[]
}

export type PlaybackCompletedPayload = {
  trackId: number
  durationPlayedMs: number
}

export type PlayerErrorPayload = {
  code: string
  message: string
}

export type LibraryChangedPayload = {
  reason: string
}

export type AppEventPayload = {
  [AppEvent.PLAYER_STATE]: PlaybackState
  [AppEvent.PLAYER_TRACK_CHANGED]: TrackChangedPayload
  [AppEvent.PLAYER_QUEUE_CHANGED]: QueueChangedPayload
  [AppEvent.PLAYER_COMPLETED]: PlaybackCompletedPayload
  [AppEvent.PLAYER_ERROR]: PlayerErrorPayload
  [AppEvent.LIBRARY_SCAN_PROGRESS]: ScanStatus
  [AppEvent.LIBRARY_CHANGED]: LibraryChangedPayload
}

export type AppEventName = keyof AppEventPayload

/** Подписка на любое событие из §6; отписка — вызвать возвращённую функцию. */
export function onAppEvent<E extends AppEventName>(
  event: E,
  cb: (payload: AppEventPayload[E]) => void,
): Promise<UnlistenFn> {
  return listen<AppEventPayload[E]>(event, (e) => {
    cb(e.payload)
  })
}

export const onPlayerState = (cb: (state: PlaybackState) => void): Promise<UnlistenFn> =>
  onAppEvent(AppEvent.PLAYER_STATE, cb)

export const onTrackChanged = (cb: (payload: TrackChangedPayload) => void): Promise<UnlistenFn> =>
  onAppEvent(AppEvent.PLAYER_TRACK_CHANGED, cb)

export const onQueueChanged = (cb: (payload: QueueChangedPayload) => void): Promise<UnlistenFn> =>
  onAppEvent(AppEvent.PLAYER_QUEUE_CHANGED, cb)

export const onPlaybackCompleted = (
  cb: (payload: PlaybackCompletedPayload) => void,
): Promise<UnlistenFn> => onAppEvent(AppEvent.PLAYER_COMPLETED, cb)

export const onPlayerError = (cb: (payload: PlayerErrorPayload) => void): Promise<UnlistenFn> =>
  onAppEvent(AppEvent.PLAYER_ERROR, cb)

export const onScanProgress = (cb: (status: ScanStatus) => void): Promise<UnlistenFn> =>
  onAppEvent(AppEvent.LIBRARY_SCAN_PROGRESS, cb)

export const onLibraryChanged = (
  cb: (payload: LibraryChangedPayload) => void,
): Promise<UnlistenFn> => onAppEvent(AppEvent.LIBRARY_CHANGED, cb)

export type { UnlistenFn }
