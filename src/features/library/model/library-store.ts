import { createSignal, untrack, type Accessor } from 'solid-js'

import {
  libraryAlbums,
  libraryArtists,
  libraryFolders,
  libraryGenres,
  libraryStats,
  libraryTracks,
  onLibraryChanged,
  toIpcError,
  type Album,
  type Artist,
  type Folder,
  type Genre,
  type LibraryStats,
  type Page,
  type Track,
  type TrackQuery,
  type TrackSort,
  type UnlistenFn,
} from '@/shared/ipc'
import { DEFAULT_SETTINGS } from '@/shared/settings'

/**
 * Кэш библиотеки на время сеанса: страницы треков/альбомов/исполнителей,
 * жанры, дерево папок, сводка и «недавно добавленные».
 * Экраны только читают сигналы и просят догрузить следующую страницу —
 * никаких запросов из разметки.
 * Событие `library:changed` сбрасывает весь кэш и поднимает `libraryVersion`.
 */

export const TRACKS_PAGE_SIZE = 200
export const GRID_PAGE_SIZE = 60
/** За сколько пикселей до конца прокрутки просим следующую страницу. */
export const SCROLL_LOAD_THRESHOLD = 400

// ─── вкладки ─────────────────────────────────────────────────────────────────

export type LibraryTab = 'songs' | 'albums' | 'artists' | 'genres' | 'folders'

export const LIBRARY_TABS = [
  'songs',
  'albums',
  'artists',
  'genres',
  'folders',
] as const satisfies readonly LibraryTab[]

const [activeTabSignal, setActiveTabSignal] = createSignal<LibraryTab>('songs')

/** Активная вкладка живёт в модуле: возврат с детального экрана её не сбрасывает. */
export const activeTab: Accessor<LibraryTab> = activeTabSignal

export function setActiveTab(tab: LibraryTab): void {
  setActiveTabSignal(tab)
}

// ─── версия библиотеки ───────────────────────────────────────────────────────

const [versionSignal, setVersionSignal] = createSignal(0)

/** Растёт на каждое `library:changed`; ключ для `createResource` на экранах. */
export const libraryVersion: Accessor<number> = versionSignal

// ─── общие состояния загрузки ────────────────────────────────────────────────

export type PagedList<T> = {
  items: Accessor<readonly T[]>
  total: Accessor<number>
  loading: Accessor<boolean>
  error: Accessor<string | null>
  /** Хотя бы одна страница уже пришла. */
  loaded: Accessor<boolean>
  hasMore: Accessor<boolean>
  /** Грузит следующую страницу. После ошибки молчит — нужен `retry`. */
  load(): void
  retry(): void
  reset(): void
}

export type Loadable<T> = {
  data: Accessor<T | null>
  loading: Accessor<boolean>
  error: Accessor<string | null>
  /** Грузит значение, если его ещё нет и предыдущая попытка не упала. */
  ensure(): void
  retry(): void
  reset(): void
}

export function createPagedList<T>(
  pageSize: number,
  fetchPage: (offset: number, limit: number) => Promise<Page<T>>,
  label = 'страница',
): PagedList<T> {
  const [items, setItems] = createSignal<readonly T[]>([])
  const [total, setTotal] = createSignal(0)
  const [loading, setLoading] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)
  const [loaded, setLoaded] = createSignal(false)

  // Растёт на каждом запросе и сбросе: ответы устаревших запросов игнорируем.
  let token = 0

  const hasMore = (): boolean => !loaded() || items().length < total()

  const request = (): void => {
    const current = ++token
    const offset = items().length

    setLoading(true)
    setError(null)

    fetchPage(offset, pageSize)
      .then((page) => {
        if (current !== token) return

        const merged = offset === 0 ? [...page.items] : [...items(), ...page.items]
        setItems(() => merged)
        // Пустая страница при завышенном `total` иначе даёт бесконечную догрузку.
        setTotal(page.items.length === 0 ? merged.length : Math.max(page.total, merged.length))
        setLoaded(true)
        setLoading(false)
      })
      .catch((raw: unknown) => {
        if (current !== token) return
        console.error(`[library] не удалось загрузить ${label}`, raw)
        setError(toIpcError(raw).message)
        setLoading(false)
      })
  }

  const load = (): void => {
    untrack(() => {
      if (loading() || error() !== null || !hasMore()) return
      request()
    })
  }

  const retry = (): void => {
    untrack(() => {
      if (loading()) return
      setError(null)
      request()
    })
  }

  const reset = (): void => {
    token += 1
    setItems(() => [])
    setTotal(0)
    setLoading(false)
    setError(null)
    setLoaded(false)
  }

  return { items, total, loading, error, loaded, hasMore, load, retry, reset }
}

function createLoadable<T>(label: string, fetchValue: () => Promise<T>): Loadable<T> {
  const [data, setData] = createSignal<T | null>(null)
  const [loading, setLoading] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)

  let token = 0

  const request = (): void => {
    const current = ++token
    setLoading(true)
    setError(null)

    fetchValue()
      .then((value) => {
        if (current !== token) return
        setData(() => value)
        setLoading(false)
      })
      .catch((raw: unknown) => {
        if (current !== token) return
        console.error(`[library] не удалось загрузить ${label}`, raw)
        setError(toIpcError(raw).message)
        setLoading(false)
      })
  }

  const ensure = (): void => {
    untrack(() => {
      if (loading() || data() !== null || error() !== null) return
      request()
    })
  }

  const retry = (): void => {
    untrack(() => {
      if (loading()) return
      request()
    })
  }

  const reset = (): void => {
    token += 1
    setData(() => null)
    setLoading(false)
    setError(null)
  }

  return { data, loading, error, ensure, retry, reset }
}

// ─── сортировка треков ───────────────────────────────────────────────────────

const [sortSignal, setSortSignal] = createSignal<TrackSort>(DEFAULT_SETTINGS.librarySort)

export const librarySort: Accessor<TrackSort> = sortSignal

/** Смена сортировки обнуляет накопленные страницы — порядок другой. */
export function setLibrarySort(sort: TrackSort): void {
  if (untrack(sortSignal) === sort) return
  setSortSignal(sort)
  tracksList.reset()
}

/** Полный `TrackQuery` из §1.6: обязательные поля заданы, остальное — точечно. */
export function trackQuery(patch: Partial<TrackQuery> = {}): TrackQuery {
  return {
    sort: 'TITLE_ASC',
    offset: 0,
    limit: TRACKS_PAGE_SIZE,
    artistId: null,
    albumId: null,
    genre: null,
    folder: null,
    favoritesOnly: false,
    ...patch,
  }
}

// ─── коллекции ───────────────────────────────────────────────────────────────

export const tracksList: PagedList<Track> = createPagedList<Track>(
  TRACKS_PAGE_SIZE,
  (offset, limit) =>
    libraryTracks(trackQuery({ sort: untrack(sortSignal), offset, limit })),
  'песни',
)

export const albumsList: PagedList<Album> = createPagedList<Album>(
  GRID_PAGE_SIZE,
  (offset, limit) => libraryAlbums(offset, limit),
  'альбомы',
)

export const artistsList: PagedList<Artist> = createPagedList<Artist>(
  GRID_PAGE_SIZE,
  (offset, limit) => libraryArtists(offset, limit),
  'исполнителей',
)

export const genresList: Loadable<Genre[]> = createLoadable('жанры', () => libraryGenres())

export const statsValue: Loadable<LibraryStats> = createLoadable('сводку библиотеки', () =>
  libraryStats(),
)

// ─── папки ───────────────────────────────────────────────────────────────────

const folderCache = new Map<string, readonly Folder[]>()
const folderInFlight = new Map<string, Promise<readonly Folder[]>>()

/** Содержимое папки с кэшем: обход дерева вверх-вниз не дёргает ядро повторно. */
export function loadFolders(parent: string | null): Promise<readonly Folder[]> {
  const key = parent ?? ''

  const cached = folderCache.get(key)
  if (cached !== undefined) return Promise.resolve(cached)

  const pending = folderInFlight.get(key)
  if (pending !== undefined) return pending

  const request = libraryFolders(parent)
    .then((folders) => {
      folderCache.set(key, folders)
      return folders
    })
    .finally(() => {
      folderInFlight.delete(key)
    })

  folderInFlight.set(key, request)
  return request
}

// ─── сброс и подписка ────────────────────────────────────────────────────────

/** Полный сброс кэша: библиотека изменилась либо пользователь просит перечитать. */
export function resetLibraryCache(): void {
  tracksList.reset()
  albumsList.reset()
  artistsList.reset()
  genresList.reset()
  statsValue.reset()
  folderCache.clear()
  folderInFlight.clear()
  setVersionSignal((value) => value + 1)
}

let subscription: Promise<UnlistenFn> | null = null

/** Идемпотентная подписка на `library:changed`; живёт весь сеанс. */
export function ensureLibrarySubscription(): void {
  if (subscription !== null) return

  const started = onLibraryChanged((payload) => {
    console.info(`[library] обновление библиотеки: ${payload.reason}`)
    resetLibraryCache()
  })

  started.catch((error: unknown) => {
    console.error('[library] не удалось подписаться на library:changed', error)
    subscription = null
  })

  subscription = started
}

// ─── скролл ──────────────────────────────────────────────────────────────────

export function isNearBottom(
  target: EventTarget | null,
  threshold: number = SCROLL_LOAD_THRESHOLD,
): boolean {
  if (!(target instanceof HTMLElement)) return false
  return target.scrollHeight - target.scrollTop - target.clientHeight <= threshold
}

/**
 * Слушает скролл в фазе перехвата: `VirtualList` скроллит собственный
 * внутренний элемент, до которого снаружи не дотянуться.
 * Возвращает функцию отписки.
 */
export function watchScrollBottom(
  root: HTMLElement,
  onReachBottom: () => void,
  threshold: number = SCROLL_LOAD_THRESHOLD,
): () => void {
  const handle = (event: Event): void => {
    if (isNearBottom(event.target, threshold)) onReachBottom()
  }

  root.addEventListener('scroll', handle, true)
  return () => {
    root.removeEventListener('scroll', handle, true)
  }
}

// ─── параметры маршрутов ─────────────────────────────────────────────────────

/** Имя жанра и путь папки содержат `/` — в сегмент маршрута уходят закодированными. */
export function encodeRouteParam(value: string): string {
  return encodeURIComponent(value)
}

export function decodeRouteParam(value: string): string {
  try {
    return decodeURIComponent(value)
  } catch {
    return value
  }
}

// ─── склонения ───────────────────────────────────────────────────────────────

export const TRACK_FORMS = ['трек', 'трека', 'треков'] as const
export const ALBUM_FORMS = ['альбом', 'альбома', 'альбомов'] as const
export const ARTIST_FORMS = ['исполнитель', 'исполнителя', 'исполнителей'] as const
export const GENRE_FORMS = ['жанр', 'жанра', 'жанров'] as const
export const FOLDER_FORMS = ['папка', 'папки', 'папок'] as const
