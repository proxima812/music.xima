import { useNavigate, useParams } from '@solidjs/router'
import { ArrowLeft, Tags } from 'lucide-solid'
import { createEffect, createSignal, Match, onCleanup, Show, Switch } from 'solid-js'

import { usePlayer } from '@/features/player/model/player-store'
import { TrackMenu } from '@/features/player/ui/TrackMenu'
import { libraryTracks, type Track } from '@/shared/ipc'
import { formatPlural } from '@/shared/lib'
import {
  IconButton,
  Screen,
  Spinner,
  TopBar,
  TRACK_ROW_HEIGHT,
  TrackRow,
  VirtualList,
} from '@/shared/ui'
import {
  createPagedList,
  decodeRouteParam,
  libraryVersion,
  TRACK_FORMS,
  TRACKS_PAGE_SIZE,
  trackQuery,
  watchScrollBottom,
} from '../model/library-store'
import { EmptyLibraryState, ErrorState, ListSkeleton, PlaybackButtons } from './LibraryStates'

/** Треки одного жанра со страничной догрузкой. */
export function GenreScreen() {
  const params = useParams<{ name: string }>()
  const navigate = useNavigate()
  const player = usePlayer()

  const [scroller, setScroller] = createSignal<HTMLDivElement | null>(null)
  const [menuTrack, setMenuTrack] = createSignal<Track | null>(null)

  const genre = (): string => decodeRouteParam(params.name)

  const list = createPagedList<Track>(
    TRACKS_PAGE_SIZE,
    (offset, limit) =>
      libraryTracks(trackQuery({ sort: 'TITLE_ASC', offset, limit, genre: genre() })),
    `треки жанра «${genre()}»`,
  )

  createEffect(() => {
    genre()
    libraryVersion()
    list.reset()
    list.load()
  })

  createEffect(() => {
    const element = scroller()
    if (element === null) return
    onCleanup(
      watchScrollBottom(element, () => {
        list.load()
      }),
    )
  })

  const playFrom = (index: number): void => {
    const items = list.items()
    if (items.length === 0) return
    player.setShuffle(false)
    player.playTracks(
      items.map((track) => track.id),
      index,
    )
  }

  const shuffleAll = (): void => {
    const items = list.items()
    if (items.length === 0) return
    player.setShuffle(true)
    player.playTracks(
      items.map((track) => track.id),
      Math.floor(Math.random() * items.length),
    )
  }

  return (
    <Screen scrollable={false}>
      <TopBar
        title={genre()}
        subtitle={formatPlural(list.total(), TRACK_FORMS)}
        left={
          <IconButton
            label="Назад"
            onClick={() => {
              navigate(-1)
            }}
          >
            <ArrowLeft size={20} aria-hidden="true" />
          </IconButton>
        }
      />

      <div class="shrink-0 px-4 pb-3">
        <PlaybackButtons
          onPlay={() => {
            playFrom(0)
          }}
          onShuffle={shuffleAll}
          disabled={list.items().length === 0}
        />
      </div>

      <Switch>
        <Match when={list.items().length === 0 && list.error() !== null}>
          <ErrorState
            message={list.error() ?? ''}
            onRetry={() => {
              list.retry()
            }}
          />
        </Match>

        <Match when={!list.loaded()}>
          <ListSkeleton />
        </Match>

        <Match when={list.items().length === 0}>
          <EmptyLibraryState
            icon={<Tags aria-hidden="true" />}
            title="В этом жанре нет треков"
            description="Возможно, библиотека изменилась — обновите её в настройках"
          />
        </Match>

        <Match when={list.items().length > 0}>
          <div ref={setScroller} class="relative min-h-0 flex-1">
            <VirtualList items={list.items()} estimateSize={TRACK_ROW_HEIGHT} class="pb-player-stack">
              {(track, index) => (
                <TrackRow
                  track={track}
                  index={index}
                  active={player.state.trackId === track.id}
                  onPlay={(_track, position) => {
                    playFrom(position ?? 0)
                  }}
                  onMenu={(selected) => {
                    setMenuTrack(selected)
                  }}
                />
              )}
            </VirtualList>

            <Show when={list.loading()}>
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
