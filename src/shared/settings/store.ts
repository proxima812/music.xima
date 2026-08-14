import { load as openStore } from '@tauri-apps/plugin-store'
import type { Store } from '@tauri-apps/plugin-store'

import { oneOf, REPEAT_MODES, TRACK_SORTS } from '../ipc/enums'
import { debounce } from '../lib/async'
import {
  cloneSettings,
  DEFAULT_SETTINGS,
  MAX_CROSSFADE_MS,
  type AppSettings,
  type LastQueue,
} from './defaults'

/**
 * Обёртка над `@tauri-apps/plugin-store` (файл `settings.json`).
 * Чтение — один раз на старте в память, запись — дебаунсом.
 * В Store живут только настройки из §9; библиотека и плейлисты — в SQLite.
 */

export const SETTINGS_FILE = 'settings.json'

const WRITE_DEBOUNCE_MS = 300

export type SettingsStore = {
  /** Читает файл в память. Повторные вызовы возвращают уже прочитанное. */
  load(): Promise<AppSettings>
  get<K extends keyof AppSettings>(key: K): AppSettings[K]
  set<K extends keyof AppSettings>(key: K, value: AppSettings[K]): void
  /** Сбрасывает отложенную запись на диск немедленно. */
  save(): Promise<void>
  /** Копия текущих значений — для инициализации сторов UI. */
  snapshot(): AppSettings
}

const state: AppSettings = cloneSettings(DEFAULT_SETTINGS)
const dirty = new Set<keyof AppSettings>()

let storePromise: Promise<Store> | null = null
let loadPromise: Promise<AppSettings> | null = null
let writeChain: Promise<void> = Promise.resolve()

function ensureStore(): Promise<Store> {
  const existing =
    storePromise ??
    openStore(SETTINGS_FILE, {
      autoSave: false,
      defaults: { ...DEFAULT_SETTINGS },
    }).catch((error: unknown) => {
      storePromise = null
      throw error
    })
  storePromise = existing
  return existing
}

async function readAll(): Promise<AppSettings> {
  try {
    const store = await ensureStore()
    const entries = await store.entries<unknown>()
    // Значения, изменённые до конца загрузки, диском не перетираем.
    const overrides: Record<string, unknown> = {}
    for (const key of dirty) overrides[key] = state[key]
    Object.assign(state, parseSettings(Object.fromEntries(entries)), overrides)
  } catch (error) {
    console.error(`[settings] не удалось прочитать ${SETTINGS_FILE}`, error)
  }
  return snapshot()
}

function load(): Promise<AppSettings> {
  const existing = loadPromise ?? readAll()
  loadPromise = existing
  return existing
}

function get<K extends keyof AppSettings>(key: K): AppSettings[K] {
  return state[key]
}

function set<K extends keyof AppSettings>(key: K, value: AppSettings[K]): void {
  state[key] = value
  dirty.add(key)
  scheduleWrite()
}

const scheduleWrite = debounce((): void => {
  void save()
}, WRITE_DEBOUNCE_MS)

function save(): Promise<void> {
  scheduleWrite.cancel()
  const next = writeChain.then(writeDirty)
  writeChain = next
  return next
}

async function writeDirty(): Promise<void> {
  if (dirty.size === 0) return

  const keys = [...dirty]
  dirty.clear()

  try {
    const store = await ensureStore()
    for (const key of keys) {
      await store.set(key, state[key])
    }
    await store.save()
  } catch (error) {
    console.error(`[settings] не удалось записать ${SETTINGS_FILE}`, error)
  }
}

function snapshot(): AppSettings {
  return cloneSettings(state)
}

export const settings: SettingsStore = { load, get, set, save, snapshot }

// ─── разбор того, что лежит на диске ─────────────────────────────────────────

function parseSettings(raw: Record<string, unknown>): AppSettings {
  return {
    volume: parseRange(raw['volume'], 0, 1) ?? DEFAULT_SETTINGS.volume,
    repeat: oneOf(raw['repeat'], REPEAT_MODES) ?? DEFAULT_SETTINGS.repeat,
    shuffle: parseBoolean(raw['shuffle']) ?? DEFAULT_SETTINGS.shuffle,
    librarySort: oneOf(raw['librarySort'], TRACK_SORTS) ?? DEFAULT_SETTINGS.librarySort,
    rememberQueue: parseBoolean(raw['rememberQueue']) ?? DEFAULT_SETTINGS.rememberQueue,
    lastQueue: parseLastQueue(raw['lastQueue']),
    scanRoots: parseStringArray(raw['scanRoots']) ?? [...DEFAULT_SETTINGS.scanRoots],
    crossfadeMs:
      parseRange(raw['crossfadeMs'], 0, MAX_CROSSFADE_MS, true) ?? DEFAULT_SETTINGS.crossfadeMs,
  }
}

function parseBoolean(value: unknown): boolean | undefined {
  return typeof value === 'boolean' ? value : undefined
}

function parseRange(value: unknown, min: number, max: number, integer = false): number | undefined {
  if (typeof value !== 'number' || !Number.isFinite(value)) return undefined
  const clamped = Math.min(Math.max(value, min), max)
  return integer ? Math.round(clamped) : clamped
}

function parseStringArray(value: unknown): string[] | undefined {
  const source = asArray(value)
  if (source === undefined) return undefined

  const items: string[] = []
  for (const item of source) {
    if (typeof item === 'string') items.push(item)
  }
  return items
}

function parseNumberArray(value: unknown): number[] | undefined {
  const source = asArray(value)
  if (source === undefined) return undefined

  const items: number[] = []
  for (const item of source) {
    if (typeof item === 'number' && Number.isFinite(item)) items.push(item)
  }
  return items
}

/** `Array.isArray` сужает `unknown` до `any[]`, поэтому возвращаем честный тип. */
function asArray(value: unknown): readonly unknown[] | undefined {
  return Array.isArray(value) ? (value as readonly unknown[]) : undefined
}

function parseLastQueue(value: unknown): LastQueue | null {
  if (typeof value !== 'object' || value === null) return null

  const record: Record<string, unknown> = value as Record<string, unknown>
  const trackIds = parseNumberArray(record['trackIds'])
  const index = parseRange(record['index'], 0, Number.MAX_SAFE_INTEGER, true)
  const positionMs = parseRange(record['positionMs'], 0, Number.MAX_SAFE_INTEGER, true)

  if (trackIds === undefined || trackIds.length === 0) return null
  if (index === undefined || positionMs === undefined) return null
  if (index >= trackIds.length) return null

  return { trackIds, index, positionMs }
}
