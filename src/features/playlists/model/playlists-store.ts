import { createSignal } from 'solid-js'

import {
  playlistAddTracks,
  playlistCreate,
  playlistDelete,
  playlistList,
  playlistRemoveAt,
  playlistRename,
  playlistReorder,
  playlistTracks,
  smartPlaylistList,
  toIpcError,
  type Playlist,
  type SmartPlaylist,
} from '@/shared/ipc'
import { toast } from '@/shared/ui'

/**
 * Списки плейлистов на весь сеанс: их читает и экран плейлистов, и боттом-шит
 * «Добавить в плейлист». После любой мутации список перечитывается, чтобы
 * `trackCount`/`updatedAt` не разъезжались с ядром.
 */

/** Ограничение имени в UI. Ядро режет длиннее, но пользователю говорим заранее. */
export const MAX_PLAYLIST_NAME_LEN = 100

const [playlists, setPlaylists] = createSignal<readonly Playlist[]>([])
const [smartPlaylists, setSmartPlaylists] = createSignal<readonly SmartPlaylist[]>([])
const [loading, setLoading] = createSignal(false)
const [error, setError] = createSignal<string | null>(null)

let loaded = false
let inFlight: Promise<void> | null = null

export function allPlaylists(): readonly Playlist[] {
  return playlists()
}

export function allSmartPlaylists(): readonly SmartPlaylist[] {
  return smartPlaylists()
}

export function playlistsLoading(): boolean {
  return loading()
}

export function playlistsError(): string | null {
  return error()
}

export function findPlaylist(id: number): Playlist | null {
  return playlists().find((playlist) => playlist.id === id) ?? null
}

/** Перечитывает оба списка. Параллельные вызовы схлопываются в один запрос. */
export function refreshPlaylists(): Promise<void> {
  const existing = inFlight ?? load()
  inFlight = existing
  return existing
}

/** Читает списки, если их ещё ни разу не читали. */
export function ensurePlaylists(): Promise<void> {
  return loaded ? Promise.resolve() : refreshPlaylists()
}

async function load(): Promise<void> {
  setLoading(true)
  try {
    const [simple, smart] = await Promise.all([playlistList(), smartPlaylistList()])
    setPlaylists(simple)
    setSmartPlaylists(smart)
    setError(null)
    loaded = true
  } catch (raw) {
    const failure = toIpcError(raw)
    console.error('[playlists] не удалось прочитать списки', failure)
    setError(failure.message)
  } finally {
    setLoading(false)
    inFlight = null
  }
}

// ─── мутации ─────────────────────────────────────────────────────────────────

/** Создаёт плейлист и, если переданы треки, сразу кладёт их внутрь. */
export async function createPlaylist(
  name: string,
  trackIds: readonly number[] = [],
): Promise<Playlist | null> {
  const value = name.trim()
  try {
    const created = await playlistCreate(value)
    if (trackIds.length > 0) {
      await playlistAddTracks(created.id, [...trackIds])
      invalidateCovers(created.id)
    }
    await refreshPlaylists()
    toast({
      title:
        trackIds.length > 0
          ? `Добавлено в «${created.name}»`
          : `Плейлист «${created.name}» создан`,
      variant: 'success',
    })
    return created
  } catch (raw) {
    report('не удалось создать плейлист', 'Не удалось создать плейлист', raw)
    return null
  }
}

export async function renamePlaylist(id: number, name: string): Promise<boolean> {
  const value = name.trim()
  try {
    await playlistRename(id, value)
    invalidateCovers(id)
    await refreshPlaylists()
    toast({ title: 'Плейлист переименован', variant: 'success' })
    return true
  } catch (raw) {
    report(`не удалось переименовать плейлист ${String(id)}`, 'Не удалось переименовать', raw)
    return false
  }
}

export async function deletePlaylist(id: number): Promise<boolean> {
  const name = findPlaylist(id)?.name
  try {
    await playlistDelete(id)
    invalidateCovers(id)
    await refreshPlaylists()
    toast({
      title: name === undefined ? 'Плейлист удалён' : `Плейлист «${name}» удалён`,
      variant: 'success',
    })
    return true
  } catch (raw) {
    report(`не удалось удалить плейлист ${String(id)}`, 'Не удалось удалить плейлист', raw)
    return false
  }
}

export async function addTracksToPlaylist(
  id: number,
  trackIds: readonly number[],
): Promise<boolean> {
  if (trackIds.length === 0) return false

  const name = findPlaylist(id)?.name
  try {
    await playlistAddTracks(id, [...trackIds])
    invalidateCovers(id)
    await refreshPlaylists()
    toast({
      title: name === undefined ? 'Добавлено в плейлист' : `Добавлено в «${name}»`,
      variant: 'success',
    })
    return true
  } catch (raw) {
    report(`не удалось добавить треки в плейлист ${String(id)}`, 'Не удалось добавить', raw)
    return false
  }
}

export async function removeTrackAt(id: number, position: number): Promise<boolean> {
  try {
    await playlistRemoveAt(id, position)
    invalidateCovers(id)
    await refreshPlaylists()
    return true
  } catch (raw) {
    report(
      `не удалось удалить трек из плейлиста ${String(id)}`,
      'Не удалось удалить трек',
      raw,
    )
    return false
  }
}

export async function reorderPlaylist(id: number, from: number, to: number): Promise<boolean> {
  if (from === to) return true

  try {
    await playlistReorder(id, from, to)
    invalidateCovers(id)
    await refreshPlaylists()
    return true
  } catch (raw) {
    report(`не удалось переставить трек в плейлисте ${String(id)}`, 'Не удалось переставить', raw)
    return false
  }
}

function report(logMessage: string, toastTitle: string, raw: unknown): void {
  const failure = toIpcError(raw)
  console.error(`[playlists] ${logMessage}`, failure)
  toast({ title: toastTitle, description: failure.message, variant: 'danger' })
}

// ─── обложки для мозаики ─────────────────────────────────────────────────────

/** Сколько обложек нужно мозаике 2x2. */
export const MOSAIC_COVERS = 4

const covers = new Map<number, Promise<readonly string[]>>()

/**
 * Ключи обложек первых треков плейлиста — на карточку-мозаику.
 * Результат кэшируется: список плейлистов перерисовывается часто.
 */
export function playlistCovers(id: number): Promise<readonly string[]> {
  const cached = covers.get(id)
  if (cached !== undefined) return cached

  const request: Promise<readonly string[]> = playlistTracks(id)
    .then((tracks): readonly string[] => {
      const keys: string[] = []
      for (const track of tracks) {
        const key = track.coverKey
        if (key === null || key === '' || keys.includes(key)) continue
        keys.push(key)
        if (keys.length >= MOSAIC_COVERS) break
      }
      return keys
    })
    .catch((raw: unknown): readonly string[] => {
      covers.delete(id)
      console.error(`[playlists] не удалось прочитать треки плейлиста ${String(id)}`, raw)
      return []
    })

  covers.set(id, request)
  return request
}

export function invalidateCovers(id?: number): void {
  if (id === undefined) covers.clear()
  else covers.delete(id)
}

// ─── имя плейлиста ───────────────────────────────────────────────────────────

/** Возвращает текст ошибки либо `null`, если имя годится. */
export function validatePlaylistName(name: string): string | null {
  const value = name.trim()
  if (value === '') return 'Введите название плейлиста'
  if (value.length > MAX_PLAYLIST_NAME_LEN) {
    return `Не длиннее ${String(MAX_PLAYLIST_NAME_LEN)} символов`
  }
  return null
}

/**
 * Локальный аналог `playlist_reorder`: элемент вынимается из позиции `from`
 * и вставляется в позицию `to` уже укороченного списка.
 */
export function moveItem<T>(items: readonly T[], from: number, to: number): T[] {
  const next = [...items]
  if (from < 0 || from >= next.length) return next

  const [moved] = next.splice(from, 1)
  if (moved === undefined) return next

  const target = Math.min(Math.max(to, 0), next.length)
  next.splice(target, 0, moved)
  return next
}
