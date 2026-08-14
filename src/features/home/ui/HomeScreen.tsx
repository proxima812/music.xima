import { useNavigate } from '@solidjs/router'
import { FolderPlus, Settings, Shuffle, Sparkles } from 'lucide-solid'
import { createEffect, createSignal, Match, onCleanup, Show, Switch } from 'solid-js'

import {
  ensureLibrarySubscription,
  libraryVersion,
  tracksList,
  TRACK_FORMS,
  watchScrollBottom,
} from '@/features/library/model/library-store'
import { ErrorState, ListSkeleton } from '@/features/library/ui/LibraryStates'
import { usePlayer } from '@/features/player/model/player-store'
import { TrackMenu } from '@/features/player/ui/TrackMenu'
import type { Track } from '@/shared/ipc'
import { formatPlural } from '@/shared/lib'
import {
  Button,
  EmptyState,
  IconButton,
  Screen,
  Spinner,
  TopBar,
  TrackRow,
  TRACK_ROW_HEIGHT,
  VirtualList,
} from '@/shared/ui'

/**
 * Главная — это вся музыка одним списком. Никаких каруселей и подборок:
 * открыл приложение, видишь треки, нажал — играет.
 */
export function HomeScreen() {
  const navigate = useNavigate()
  const player = usePlayer()

  const [scroller, setScroller] = createSignal<HTMLDivElement | null>(null)
  const [menuTrack, setMenuTrack] = createSignal<Track | null>(null)

  createEffect(() => {
    libraryVersion()
    ensureLibrarySubscription()
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
    player.setShuffle(false)
    player.playTracks(
      items.map((track) => track.id),
      index ?? 0,
    )
  }

  const shuffleAll = (): void => {
    const items = tracksList.items()
    if (items.length === 0) return
    player.setShuffle(true)
    player.playTracks(
      items.map((track) => track.id),
      Math.floor(Math.random() * items.length),
    )
  }

  const isCurrent = (track: Track): boolean => player.state.trackId === track.id

  return (
    <Screen scrollable={false}>
      <TopBar
        title="music.xima"
        right={
          <>
            <IconButton
              label="Умные плейлисты"
              onClick={() => {
                navigate('/smart')
              }}
            >
              <Sparkles aria-hidden="true" />
            </IconButton>
            <IconButton
              label="Настройки"
              onClick={() => {
                navigate('/settings')
              }}
            >
              <Settings aria-hidden="true" />
            </IconButton>
          </>
        }
      />

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
          <EmptyState
            class="min-h-[60vh]"
            icon={<FolderPlus aria-hidden="true" />}
            title="Библиотека пуста"
            description="Добавьте папку с музыкой — приложение просканирует её и соберёт библиотеку."
            action={
              <Button
                variant="primary"
                onClick={() => {
                  navigate('/settings')
                }}
              >
                Добавить папку
              </Button>
            }
          />
        </Match>

        <Match when={tracksList.items().length > 0}>
          <div class="flex min-h-11 shrink-0 items-center justify-between gap-3 px-4 pb-2">
            <span class="truncate text-xs text-muted tabular-nums">
              {formatPlural(tracksList.total(), TRACK_FORMS)}
            </span>
            <Button size="sm" variant="tertiary" onClick={shuffleAll}>
              <Shuffle size={16} aria-hidden="true" />
              Перемешать
            </Button>
          </div>

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
    </Screen>
  )
}
