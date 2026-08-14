import {
  createContext,
  createSignal,
  useContext,
  type Accessor,
  type ParentProps,
} from 'solid-js'

import { PlayerProvider } from '@/features/player/model/player-store'
import { cloneSettings, DEFAULT_SETTINGS, settings, type AppSettings } from '@/shared/settings'
import { Toasts } from '@/shared/ui'

export type SettingsContextValue = {
  /** Текущее состояние настроек; реактивно. */
  value: Accessor<AppSettings>
  /** `false`, пока settings.json ещё читается с диска. */
  ready: Accessor<boolean>
  /** Пишет в память и в Store (там запись дебаунсом). */
  set: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => void
}

const SettingsContext = createContext<SettingsContextValue>()

/** Настройки приложения: читаются один раз на старте, пишутся через C0-хранилище. */
export function SettingsProvider(props: ParentProps) {
  const [value, setValue] = createSignal<AppSettings>(cloneSettings(DEFAULT_SETTINGS))
  const [ready, setReady] = createSignal(false)

  void settings
    .load()
    .then((loaded) => {
      setValue(loaded)
    })
    .catch((error: unknown) => {
      console.error('[settings] не удалось загрузить настройки', error)
    })
    .finally(() => {
      setReady(true)
    })

  const set = <K extends keyof AppSettings>(key: K, next: AppSettings[K]): void => {
    const updated = cloneSettings(value())
    updated[key] = next
    setValue(updated)
    settings.set(key, next)
  }

  return (
    <SettingsContext.Provider value={{ value, ready, set }}>
      {props.children}
    </SettingsContext.Provider>
  )
}

export function useSettings(): SettingsContextValue {
  const context = useContext(SettingsContext)
  if (context === undefined) {
    throw new Error('useSettings вызван вне SettingsProvider')
  }
  return context
}

/** Все провайдеры приложения плюс регион тостов. */
export function AppProviders(props: ParentProps) {
  return (
    <SettingsProvider>
      <PlayerProvider>
        {props.children}
        <Toasts />
      </PlayerProvider>
    </SettingsProvider>
  )
}
