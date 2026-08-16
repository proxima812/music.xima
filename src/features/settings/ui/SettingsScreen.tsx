import { useNavigate } from '@solidjs/router'
import { ChevronLeft, ChevronRight } from 'lucide-solid'

import { useSettings } from '@/app/providers'
import { playerSetCrossfade, playerSetShuffle, playerSetVolume } from '@/shared/ipc'
import { MAX_CROSSFADE_MS } from '@/shared/settings'
import { IconButton, Screen, SectionHeader, Slider, Switch, TopBar } from '@/shared/ui'
import { LibrarySection } from './LibrarySection'

/** Экран настроек: папки библиотеки и воспроизведение. Тема одна — тёмная. */
export function SettingsScreen() {
  const navigate = useNavigate()
  const settings = useSettings()

  const volumePercent = (): number => Math.round(settings.value().volume * 100)

  const setVolume = (percent: number): void => {
    settings.set('volume', Math.min(1, Math.max(0, percent / 100)))
  }

  const pushVolume = (percent: number): void => {
    void playerSetVolume(Math.min(1, Math.max(0, percent / 100))).catch((error: unknown) => {
      console.error('[settings] громкость не доехала до плеера', error)
    })
  }

  const crossfadeSeconds = (): number => settings.value().crossfadeMs / 1000

  const crossfadeLabel = (): string => {
    const seconds = crossfadeSeconds()
    return seconds === 0 ? 'Выкл.' : `${seconds.toFixed(1)} с`
  }

  const setCrossfade = (seconds: number): void => {
    settings.set('crossfadeMs', Math.round(seconds * 1000))
  }

  const pushCrossfade = (seconds: number): void => {
    void playerSetCrossfade(Math.round(seconds * 1000)).catch((error: unknown) => {
      console.error('[settings] плавный переход не доехал до плеера', error)
    })
  }

  const setShuffle = (enabled: boolean): void => {
    settings.set('shuffle', enabled)
    void playerSetShuffle(enabled).catch((error: unknown) => {
      console.error('[settings] shuffle не доехал до плеера', error)
    })
  }

  return (
    <Screen>
      <TopBar
        title="Настройки"
        left={
          <IconButton
            label="Назад"
            onClick={() => {
              navigate(-1)
            }}
          >
            <ChevronLeft aria-hidden="true" />
          </IconButton>
        }
      />

      <div class="flex flex-col gap-2 pb-8">
        <LibrarySection />

        <section aria-label="Библиотека" class="px-4">
          <div class="depth-raised overflow-hidden rounded-2xl">
            <button
              type="button"
              class="flex min-h-14 w-full items-center gap-3 px-4 text-start no-highlight"
              onClick={() => {
                navigate('/settings/hidden-tracks')
              }}
            >
              <span class="min-w-0 flex-1 truncate text-sm text-foreground">Скрытые песни</span>
              <ChevronRight class="size-5 shrink-0 text-muted" aria-hidden="true" />
            </button>
          </div>
        </section>

        <section>
          <SectionHeader title="Воспроизведение" />

          <div class="flex flex-col gap-4 px-4">
            <Switch
              checked={settings.value().rememberQueue}
              onChange={(checked) => {
                settings.set('rememberQueue', checked)
              }}
              label="Помнить очередь"
              description="После перезапуска приложение вернёт очередь и позицию трека"
            />

            <Switch
              checked={settings.value().shuffle}
              onChange={setShuffle}
              label="Перемешивание"
              description="Новая очередь запускается в случайном порядке"
            />

            <Slider
              label="Громкость"
              valueLabel={`${volumePercent()}%`}
              ariaLabel="Громкость"
              value={volumePercent()}
              min={0}
              max={100}
              step={1}
              onChange={setVolume}
              onChangeEnd={pushVolume}
            />

            <div class="flex flex-col gap-1">
              <Slider
                label="Плавный переход"
                valueLabel={crossfadeLabel()}
                ariaLabel="Плавный переход между треками"
                value={crossfadeSeconds()}
                min={0}
                max={MAX_CROSSFADE_MS / 1000}
                step={0.5}
                onChange={setCrossfade}
                onChangeEnd={pushCrossfade}
              />
              <p class="text-xs text-muted">
                Конец трека плавно затихает, следующий так же плавно нарастает
              </p>
            </div>
          </div>
        </section>
      </div>
    </Screen>
  )
}
