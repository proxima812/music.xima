import { createEffect, createSignal, Match, onCleanup, Show, Switch } from 'solid-js'

import { useSettings } from '@/app/providers'
import { usePlayer } from '@/features/player/model/player-store'
import { TrackMenu } from '@/features/player/ui/TrackMenu'
import type { Track } from '@/shared/ipc'
import { formatPlural } from '@/shared/lib'
import { Spinner, TRACK_ROW_HEIGHT, TrackRow, VirtualList } from '@/shared/ui'
import {
  librarySort,
  libraryVersion,
  setLibrarySort,
  TRACK_FORMS,
  tracksList,
  watchScrollBottom,
} from '../model/library-store'
import { EmptyLibraryState, ErrorState, ListSkeleton } from './LibraryStates'
import { SortMenu } from './SortMenu'

/** Все песни: виртуализованный список со страничной догрузкой и сортировкой. */
export function SongsTab() {
  const player = usePlayer()
  const settings = useSettings()

  const [scroller, setScroller] = createSignal<HTMLDivElement | null>(null)
  const [menuTrack, setMenuTrack] = createSignal<Track | null>(null)

  // Настройка — источник истины по сортировке, стор только следует за ней.
  createEffect(() => {
    setLibrarySort(settings.value().librarySort)
  })

  // Первая страница: на монтировании, после смены сортировки и после library:changed.
  createEffect(() => {
    librarySort()
    libraryVersion()
    tracksList.load()
  })

  createEffect(() => {
    const element = scroller()
    if (element === null) return
    onCleanup(
      watchScrollBottom(element, () => {
        tracksList.load()
      }),
    )
  })

  const playFrom = (index: number | undefined): void => {
    const items = tracksList.items()
    if (items.length === 0) return
    player.playTracks(
      items.map((track) => track.id),
      index ?? 0,
    )
  }

  const isCurrent = (track: Track): boolean => player.state.trackId === track.id

  return (
    <div class="flex h-full min-h-0 flex-col">
      <div class="flex min-h-11 shrink-0 items-center justify-between gap-2 px-4">
        <span class="truncate text-xs text-muted">
          {formatPlural(tracksList.total(), TRACK_FORMS)}
        </span>
        <SortMenu
          value={settings.value().librarySort}
          onChange={(sort) => {
            settings.set('librarySort', sort)
          }}
        />
      </div>

      <Switch>
        <Match when={tracksList.items().length === 0 && tracksList.error() !== null}>
          <ErrorState
            message={tracksList.error() ?? ''}
            onRetry={() => {
              tracksList.retry()
            }}
          />
        </Match>

        <Match when={!tracksList.loaded()}>
          <ListSkeleton />
        </Match>

        <Match when={tracksList.items().length === 0}>
          <EmptyLibraryState
            title="Песен пока нет"
            description="Добавьте папку с музыкой в настройках"
          />
        </Match>

        <Match when={tracksList.items().length > 0}>
          <div ref={setScroller} class="relative min-h-0 flex-1">
            <VirtualList
              items={tracksList.items()}
              estimateSize={TRACK_ROW_HEIGHT}
              class="pb-player-stack"
            >
              {(track, index) => (
                <TrackRow
                  track={track}
                  index={index}
                  active={isCurrent(track)}
                  onPlay={(_track, position) => {
                    playFrom(position)
                  }}
                  onMenu={(selected) => {
                    setMenuTrack(selected)
                  }}
                />
              )}
            </VirtualList>

            <Show when={tracksList.loading()}>
              <div class="pointer-events-none absolute inset-x-0 bottom-0 flex justify-center pb-2">
                <Spinner size="sm" color="accent" />
              </div>
            </Show>
          </div>
        </Match>
      </Switch>

      <TrackMenu
        track={menuTrack()}
        open={menuTrack() !== null}
        onOpenChange={(open) => {
          if (!open) setMenuTrack(null)
        }}
      />
    </div>
  )
}
