import { useNavigate } from '@solidjs/router'
import { ChevronLeft, EyeOff } from 'lucide-solid'
import { createResource, createSignal, For, Match, Switch } from 'solid-js'

import { toIpcError, trackHidden, trackRestore, type HiddenTrack } from '@/shared/ipc'
import { settled } from '@/shared/lib'
import { Button, EmptyState, IconButton, Screen, Spinner, TopBar, toast } from '@/shared/ui'

/** Скрытые песни: здесь можно только вернуть их в библиотеку. */
export function HiddenTracksScreen() {
  const navigate = useNavigate()
  const [hiddenTracks, { mutate, refetch }] = createResource(trackHidden)
  const [restoringId, setRestoringId] = createSignal<number | null>(null)

  // Значение читаем зеркалом: прямое чтение ресурса в загрузке поднимает общий
  // `<Suspense>`, и возврат песни вынимал бы экран из DOM (docs/BUGS.md, B8).
  const hidden = settled(hiddenTracks)

  const items = (): HiddenTrack[] =>
    [...(hidden() ?? [])].sort((left, right) => right.hiddenAt - left.hiddenAt)

  const restore = (item: HiddenTrack): void => {
    if (restoringId() !== null) return

    setRestoringId(item.track.id)
    void trackRestore(item.track.id)
      .then(() => {
        mutate((current) => current?.filter(({ track }) => track.id !== item.track.id))
        toast({ title: 'Песня возвращена', variant: 'success' })
        void refetch()
      })
      .catch((error: unknown) => {
        console.error('[settings] не удалось вернуть скрытую песню', error)
        toast({
          title: 'Не удалось вернуть песню',
          description: toIpcError(error).message,
          variant: 'danger',
        })
      })
      .finally(() => {
        setRestoringId(null)
      })
  }

  return (
    <Screen>
      <TopBar
        title="Скрытые песни"
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

      <Switch>
        <Match when={hiddenTracks.loading && hidden() === undefined}>
          <div class="flex flex-1 items-center justify-center">
            <Spinner label="Загрузка скрытых песен" />
          </div>
        </Match>

        <Match when={hiddenTracks.error !== undefined && hidden() === undefined}>
          <EmptyState
            icon={<EyeOff aria-hidden="true" />}
            title="Не удалось загрузить скрытые песни"
            action={
              <Button variant="secondary" onClick={() => void refetch()}>
                Повторить
              </Button>
            }
          />
        </Match>

        <Match when={items().length === 0}>
          <EmptyState icon={<EyeOff aria-hidden="true" />} title="Скрытых песен нет" />
        </Match>

        <Match when={items().length > 0}>
          <div class="px-4 py-2">
            <ul class="depth-raised divide-y divide-separator overflow-hidden rounded-2xl">
              <For each={items()}>
                {(item) => (
                  <li class="flex min-h-14 items-center gap-3 px-3 py-2">
                    <span class="min-w-0 flex-1">
                      <span class="block truncate text-sm font-medium text-foreground">
                        {item.track.title}
                      </span>
                      <span class="block truncate text-xs text-muted">
                        {item.track.artistName ?? 'Неизвестный исполнитель'}
                      </span>
                    </span>
                    <Button
                      variant="ghost"
                      size="sm"
                      class="min-h-11 shrink-0"
                      pending={restoringId() === item.track.id}
                      disabled={restoringId() !== null && restoringId() !== item.track.id}
                      onClick={() => restore(item)}
                    >
                      Вернуть
                    </Button>
                  </li>
                )}
              </For>
            </ul>
          </div>
        </Match>
      </Switch>
    </Screen>
  )
}
