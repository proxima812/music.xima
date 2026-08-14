import {
  createContext,
  createEffect,
  createMemo,
  createSignal,
  on,
  onCleanup,
  onMount,
  untrack,
  useContext,
  type ParentProps,
} from 'solid-js'
import { createStore } from 'solid-js/store'

import {
  libraryTrack,
  onPlayerError,
  onPlayerState,
  onQueueChanged,
  onTrackChanged,
  playerAddNext,
  playerAddToQueue,
  playerNext,
  playerPause,
  playerPlay,
  playerPrevious,
  playerQueue,
  playerSeek,
  playerSetQueue,
  playerSetRepeat,
  playerSetShuffle,
  playerState,
  playerToggle,
  type PlaybackState,
  type RepeatMode,
  type Track,
  type UnlistenFn,
} from '@/shared/ipc'
import { debounce } from '@/shared/lib'
import { settings } from '@/shared/settings'
import { toast } from '@/shared/ui/Toasts'

/**
 * Единственный источник истины о воспроизведении на фронте.
 * Сам ничего не вычисляет: складывает в стор то, что приходит событием
 * `player:state`. Между событиями позиция интерполируется локально (rAF)
 * и сбрасывается на каждом входящем состоянии.
 */

export type PlayerApi = {
  readonly state: PlaybackState
  readonly queue: readonly Track[]
  readonly current: Track | null
  readonly isFullOpen: boolean
  play(): void
  pause(): void
  toggle(): void
  next(): void
  prev(): void
  seek(positionMs: number): void
  playTracks(trackIds: number[], startIndex?: number): void
  setShuffle(enabled: boolean): void
  setRepeat(mode: RepeatMode): void
  addNext(trackIds: number[]): void
  addToQueue(trackIds: number[]): void
  openFull(): void
  closeFull(): void
}

export const IDLE_PLAYBACK_STATE: PlaybackState = {
  status: 'IDLE',
  trackId: null,
  positionMs: 0,
  durationMs: 0,
  queueIndex: null,
  queueLength: 0,
  shuffle: false,
  repeat: 'OFF',
  volume: 1,
  speed: 1,
}

/** Шаг записи интерполированной позиции: 200 мс не видно глазу, но не жжёт CPU. */
const POSITION_STEP_MS = 200

const LAST_QUEUE_DEBOUNCE_MS = 1000

const PlayerContext = createContext<PlayerApi>()

export function PlayerProvider(props: ParentProps) {
  const [state, setState] = createStore<PlaybackState>({ ...IDLE_PLAYBACK_STATE })
  const [queue, setQueue] = createSignal<readonly Track[]>([])
  const [detached, setDetached] = createSignal<Track | null>(null)
  const [isFullOpen, setIsFullOpen] = createSignal(false)

  // База локальной интерполяции позиции; сбрасывается каждым событием и seek-ом.
  let baseAt = 0
  let basePosition = 0
  let frame: number | null = null

  const rebase = (positionMs: number): void => {
    basePosition = positionMs
    baseAt = performance.now()
  }

  const applyState = (next: PlaybackState): void => {
    rebase(next.positionMs)
    setState(next)
  }

  const tick = (): void => {
    frame = null
    if (state.status !== 'PLAYING') return

    const speed = state.speed > 0 ? state.speed : 1
    const limit = state.durationMs > 0 ? state.durationMs : Number.MAX_SAFE_INTEGER
    const interpolated = Math.min(basePosition + (performance.now() - baseAt) * speed, limit)

    if (Math.abs(interpolated - state.positionMs) >= POSITION_STEP_MS) {
      setState('positionMs', Math.round(interpolated))
    }
    frame = requestAnimationFrame(tick)
  }

  createEffect(() => {
    const running = state.status === 'PLAYING'
    if (running && frame === null) {
      frame = requestAnimationFrame(tick)
      return
    }
    if (!running && frame !== null) {
      cancelAnimationFrame(frame)
      frame = null
    }
  })

  onCleanup(() => {
    if (frame !== null) cancelAnimationFrame(frame)
    frame = null
  })

  // ─── синхронизация с ядром ─────────────────────────────────────────────────

  const refreshQueue = async (): Promise<void> => {
    try {
      setQueue(await playerQueue())
    } catch (error) {
      console.error('[player] не удалось прочитать очередь', error)
    }
  }

  const resync = async (): Promise<void> => {
    try {
      applyState(await playerState())
    } catch (error) {
      console.error('[player] не удалось прочитать состояние', error)
    }
  }

  /** Оптимистичное значение уже в сторе: при ошибке пишем в лог и перечитываем правду. */
  const perform = (message: string, action: () => Promise<void>): void => {
    action().catch((error: unknown) => {
      console.error(`[player] ${message}`, error)
      toast({ title: message, description: describeError(error), variant: 'danger' })
      void resync()
    })
  }

  // ─── текущий трек ──────────────────────────────────────────────────────────

  const current = createMemo<Track | null>(() => {
    const id = state.trackId
    if (id === null) return null

    const items = queue()
    const index = state.queueIndex
    if (index !== null && index >= 0 && index < items.length) {
      const at = items[index]
      if (at !== undefined && at.id === id) return at
    }

    const found = items.find((track) => track.id === id)
    if (found !== undefined) return found

    const fallback = detached()
    return fallback !== null && fallback.id === id ? fallback : null
  })

  // Трек может играть, а очередь ещё не доехать — дотягиваем его отдельно.
  createEffect(() => {
    const id = state.trackId
    if (id === null) return
    if (untrack(current) !== null) return

    let cancelled = false
    onCleanup(() => {
      cancelled = true
    })

    libraryTrack(id)
      .then((track) => {
        if (!cancelled) setDetached(track)
      })
      .catch((error: unknown) => {
        console.error(`[player] не удалось прочитать трек ${id}`, error)
      })
  })

  // ─── действия ──────────────────────────────────────────────────────────────

  const play = (): void => {
    setState('status', 'PLAYING')
    rebase(state.positionMs)
    perform('Не удалось возобновить воспроизведение', playerPlay)
  }

  const pause = (): void => {
    setState('status', 'PAUSED')
    perform('Не удалось поставить на паузу', playerPause)
  }

  const toggle = (): void => {
    const playing = state.status === 'PLAYING' || state.status === 'BUFFERING'
    if (playing) {
      setState('status', 'PAUSED')
      perform('Не удалось поставить на паузу', playerToggle)
      return
    }
    setState('status', 'PLAYING')
    rebase(state.positionMs)
    perform('Не удалось начать воспроизведение', playerToggle)
  }

  const next = (): void => {
    setState('positionMs', 0)
    rebase(0)
    perform('Не удалось переключить трек', playerNext)
  }

  const prev = (): void => {
    setState('positionMs', 0)
    rebase(0)
    perform('Не удалось переключить трек', playerPrevious)
  }

  const seek = (positionMs: number): void => {
    const limit = state.durationMs > 0 ? state.durationMs : positionMs
    const target = Math.round(clamp(positionMs, 0, limit))
    setState('positionMs', target)
    rebase(target)
    perform('Не удалось перемотать', () => playerSeek(target))
  }

  const playTracks = (trackIds: number[], startIndex = 0): void => {
    if (trackIds.length === 0) return

    const index = Math.round(clamp(startIndex, 0, trackIds.length - 1))
    setState({
      status: 'BUFFERING',
      trackId: trackIds[index] ?? null,
      positionMs: 0,
      queueIndex: index,
      queueLength: trackIds.length,
    })
    rebase(0)

    // Запуск с любого списка — это «покажи мне, что играет»: разворачиваем плеер
    // сразу, не дожидаясь ответа нативного слоя, иначе экран прыгает с задержкой.
    setIsFullOpen(true)

    perform('Не удалось запустить воспроизведение', async () => {
      await playerSetQueue(trackIds, index, true)
      await refreshQueue()
      await resync()
    })
  }

  const setShuffle = (enabled: boolean): void => {
    setState('shuffle', enabled)
    settings.set('shuffle', enabled)
    perform('Не удалось изменить режим перемешивания', () => playerSetShuffle(enabled))
  }

  const setRepeat = (mode: RepeatMode): void => {
    setState('repeat', mode)
    settings.set('repeat', mode)
    perform('Не удалось изменить режим повтора', () => playerSetRepeat(mode))
  }

  const addNext = (trackIds: number[]): void => {
    if (trackIds.length === 0) return
    perform('Не удалось добавить в очередь', async () => {
      await playerAddNext(trackIds)
      await refreshQueue()
    })
  }

  const addToQueue = (trackIds: number[]): void => {
    if (trackIds.length === 0) return
    perform('Не удалось добавить в очередь', async () => {
      await playerAddToQueue(trackIds)
      await refreshQueue()
    })
  }

  const openFull = (): void => {
    setIsFullOpen(true)
  }

  const closeFull = (): void => {
    setIsFullOpen(false)
  }

  // ─── сохранение последней очереди ──────────────────────────────────────────

  const saveLastQueue = (): void => {
    if (!settings.get('rememberQueue')) return

    const items = untrack(queue)
    if (items.length === 0) {
      settings.set('lastQueue', null)
      return
    }

    const index = Math.round(clamp(state.queueIndex ?? 0, 0, items.length - 1))
    settings.set('lastQueue', {
      trackIds: items.map((track) => track.id),
      index,
      positionMs: Math.max(0, Math.round(state.positionMs)),
    })
  }

  const scheduleSaveLastQueue = debounce(saveLastQueue, LAST_QUEUE_DEBOUNCE_MS)

  createEffect(
    on(
      [queue, () => state.trackId, () => state.queueIndex],
      () => {
        scheduleSaveLastQueue()
      },
      { defer: true },
    ),
  )

  onCleanup(() => {
    scheduleSaveLastQueue.cancel()
  })

  // ─── старт ─────────────────────────────────────────────────────────────────

  const restoreLastQueue = async (initial: PlaybackState): Promise<void> => {
    const saved = await settings.load()
    if (!saved.rememberQueue) return

    const last = saved.lastQueue
    if (last === null || last.trackIds.length === 0) return

    // Нативный плеер мог пережить перезапуск WebView — его очередь важнее.
    if (initial.queueLength > 0 || initial.trackId !== null) return

    try {
      await playerSetQueue(last.trackIds, last.index, false)
      if (last.positionMs > 0) await playerSeek(last.positionMs)
      if (saved.shuffle !== initial.shuffle) await playerSetShuffle(saved.shuffle)
      if (saved.repeat !== initial.repeat) await playerSetRepeat(saved.repeat)
      await refreshQueue()
      await resync()
    } catch (error) {
      console.error('[player] не удалось восстановить очередь', error)
    }
  }

  const bootstrap = async (): Promise<void> => {
    let initial: PlaybackState = IDLE_PLAYBACK_STATE
    try {
      initial = await playerState()
      applyState(initial)
    } catch (error) {
      console.error('[player] не удалось прочитать состояние', error)
    }

    await refreshQueue()
    await restoreLastQueue(initial)
  }

  onMount(() => {
    void bootstrap()

    const subscriptions: Promise<UnlistenFn>[] = [
      onPlayerState(applyState),
      onQueueChanged(() => {
        void refreshQueue()
      }),
      onTrackChanged(() => {
        scheduleSaveLastQueue()
      }),
      onPlayerError((payload) => {
        console.error(`[player] ${payload.code}: ${payload.message}`)
        toast({
          title: 'Ошибка воспроизведения',
          description: payload.message,
          variant: 'danger',
        })
      }),
    ]

    const flush = (): void => {
      if (document.visibilityState !== 'hidden') return
      scheduleSaveLastQueue.cancel()
      saveLastQueue()
      void settings.save()
    }
    document.addEventListener('visibilitychange', flush)

    onCleanup(() => {
      document.removeEventListener('visibilitychange', flush)
      for (const subscription of subscriptions) {
        subscription
          .then((unlisten) => {
            unlisten()
          })
          .catch((error: unknown) => {
            console.error('[player] не удалось отписаться от события', error)
          })
      }
    })
  })

  const api: PlayerApi = {
    get state() {
      return state
    },
    get queue() {
      return queue()
    },
    get current() {
      return current()
    },
    get isFullOpen() {
      return isFullOpen()
    },
    play,
    pause,
    toggle,
    next,
    prev,
    seek,
    playTracks,
    setShuffle,
    setRepeat,
    addNext,
    addToQueue,
    openFull,
    closeFull,
  }

  return <PlayerContext.Provider value={api}>{props.children}</PlayerContext.Provider>
}

export function usePlayer(): PlayerApi {
  const api = useContext(PlayerContext)
  if (api === undefined) throw new Error('usePlayer() вызван вне <PlayerProvider>')
  return api
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min
  return Math.min(Math.max(value, min), max)
}

function describeError(error: unknown): string {
  if (error instanceof Error) return error.message
  return typeof error === 'string' ? error : 'Неизвестная ошибка'
}
