import { load as openStore } from '@tauri-apps/plugin-store'
import type { Store } from '@tauri-apps/plugin-store'
import { createSignal } from 'solid-js'

import { searchAll, toIpcError, type SearchResults } from '@/shared/ipc'
import { debounce } from '@/shared/lib'

/**
 * Состояние экрана поиска и история запросов.
 *
 * История лежит в отдельном файле Store — в `settings.json` по контракту (§9)
 * ей места нет. Результаты не кэшируются: SQLite с FTS5 отвечает быстрее, чем
 * пользователь успевает набрать следующую букву.
 */

export const SEARCH_HISTORY_FILE = 'search-history.json'

const HISTORY_KEY = 'queries'

/** Сколько запросов помним. */
export const MAX_SEARCH_HISTORY = 10

/** Сколько элементов просим в каждой секции. */
export const SEARCH_LIMIT = 20

const SEARCH_DEBOUNCE_MS = 200

export const EMPTY_SEARCH_RESULTS: SearchResults = {
  tracks: [],
  albums: [],
  artists: [],
  playlists: [],
}

// ─── история запросов ────────────────────────────────────────────────────────

const [history, setHistory] = createSignal<readonly string[]>([])

let storePromise: Promise<Store> | null = null
let loadPromise: Promise<readonly string[]> | null = null
let writeChain: Promise<void> = Promise.resolve()

function ensureStore(): Promise<Store> {
  const existing =
    storePromise ??
    openStore(SEARCH_HISTORY_FILE, { autoSave: false }).catch((error: unknown) => {
      storePromise = null
      throw error
    })
  storePromise = existing
  return existing
}

/** Последние запросы, свежие сверху. */
export function searchHistory(): readonly string[] {
  return history()
}

/** Читает историю с диска один раз за сеанс. */
export function loadSearchHistory(): Promise<readonly string[]> {
  const existing = loadPromise ?? readHistory()
  loadPromise = existing
  return existing
}

async function readHistory(): Promise<readonly string[]> {
  try {
    const store = await ensureStore()
    setHistory(parseHistory(await store.get<unknown>(HISTORY_KEY)))
  } catch (error) {
    console.error(`[search] не удалось прочитать ${SEARCH_HISTORY_FILE}`, error)
  }
  return history()
}

/** Кладёт запрос в начало истории, схлопывая повторы без учёта регистра. */
export function rememberQuery(query: string): void {
  const value = query.trim()
  if (value === '') return

  const key = fold(value)
  const rest = history().filter((item) => fold(item) !== key)
  setHistory([value, ...rest].slice(0, MAX_SEARCH_HISTORY))
  persist()
}

export function clearSearchHistory(): void {
  setHistory([])
  persist()
}

function persist(): void {
  const snapshot = [...history()]
  writeChain = writeChain.then(async () => {
    try {
      const store = await ensureStore()
      await store.set(HISTORY_KEY, snapshot)
      await store.save()
    } catch (error) {
      console.error(`[search] не удалось записать ${SEARCH_HISTORY_FILE}`, error)
    }
  })
}

function fold(value: string): string {
  return value.toLocaleLowerCase('ru-RU')
}

function parseHistory(raw: unknown): readonly string[] {
  if (!Array.isArray(raw)) return []

  const source: readonly unknown[] = raw as readonly unknown[]
  const items: string[] = []
  const seen = new Set<string>()

  for (const item of source) {
    if (typeof item !== 'string') continue

    const value = item.trim()
    if (value === '') continue

    const key = fold(value)
    if (seen.has(key)) continue

    seen.add(key)
    items.push(value)
    if (items.length >= MAX_SEARCH_HISTORY) break
  }

  return items
}

// ─── состояние экрана ────────────────────────────────────────────────────────

export type SearchStore = {
  /** Текст в поле — обновляется на каждый ввод. */
  query(): string
  /** `null` — запроса ещё не было (поле пустое). */
  results(): SearchResults | null
  loading(): boolean
  error(): string | null
  /** Ввод пользователя: дебаунсит запрос, пустую строку не отправляет. */
  setQuery(value: string): void
  /** Повторяет запрос немедленно и пишет его в историю (Enter, тап по истории). */
  submit(value?: string): void
  clear(): void
  /** Снимает отложенный запрос — вызывается из `onCleanup` экрана. */
  dispose(): void
}

export function createSearchStore(): SearchStore {
  const [query, setQueryValue] = createSignal('')
  const [results, setResults] = createSignal<SearchResults | null>(null)
  const [loading, setLoading] = createSignal(false)
  const [error, setError] = createSignal<string | null>(null)

  // Ответы приходят вразнобой: показываем только последний запрошенный.
  let generation = 0

  const reset = (): void => {
    generation += 1
    setResults(null)
    setLoading(false)
    setError(null)
  }

  const run = (value: string): void => {
    const text = value.trim()
    if (text === '') {
      reset()
      return
    }

    generation += 1
    const id = generation
    setLoading(true)
    setError(null)

    searchAll(text, SEARCH_LIMIT)
      .then((next) => {
        if (id !== generation) return
        setResults(next)
        setLoading(false)
      })
      .catch((raw: unknown) => {
        if (id !== generation) return
        const failure = toIpcError(raw)
        console.error('[search] запрос не выполнен', failure)
        setResults(null)
        setLoading(false)
        setError(failure.message)
      })
  }

  const schedule = debounce(run, SEARCH_DEBOUNCE_MS)

  const setQuery = (value: string): void => {
    setQueryValue(value)

    if (value.trim() === '') {
      schedule.cancel()
      reset()
      return
    }
    schedule(value)
  }

  const submit = (value?: string): void => {
    const text = value ?? query()
    schedule.cancel()
    setQueryValue(text)

    if (text.trim() === '') {
      reset()
      return
    }

    rememberQuery(text)
    run(text)
  }

  const clear = (): void => {
    schedule.cancel()
    setQueryValue('')
    reset()
  }

  return {
    query,
    results,
    loading,
    error,
    setQuery,
    submit,
    clear,
    dispose: () => {
      schedule.cancel()
    },
  }
}

/** В результатах вообще ничего нет. */
export function isEmptyResults(results: SearchResults): boolean {
  return (
    results.tracks.length === 0 &&
    results.albums.length === 0 &&
    results.artists.length === 0 &&
    results.playlists.length === 0
  )
}
