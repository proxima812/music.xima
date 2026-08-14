import { useNavigate } from '@solidjs/router'
import { ChevronLeft } from 'lucide-solid'

import { useSettings } from '@/app/providers'
import { playerSetShuffle, playerSetVolume } from '@/shared/ipc'
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
          </div>
        </section>
      </div>
    </Screen>
  )
}
