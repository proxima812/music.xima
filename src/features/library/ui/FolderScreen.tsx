import { useNavigate, useParams } from '@solidjs/router'
import { ArrowLeft, FolderOpen } from 'lucide-solid'
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

/** Последний сегмент display-пути: `Music/MyMusic/Rock` → `Rock`. */
function folderName(path: string): string {
  const segments = path.split('/').filter((segment) => segment !== '')
  return segments[segments.length - 1] ?? path
}

/** Треки одной папки со страничной догрузкой. */
export function FolderScreen() {
  const params = useParams<{ path: string }>()
  const navigate = useNavigate()
  const player = usePlayer()

  const [scroller, setScroller] = createSignal<HTMLDivElement | null>(null)
  const [menuTrack, setMenuTrack] = createSignal<Track | null>(null)

  const path = (): string => decodeRouteParam(params.path)

  const list = createPagedList<Track>(
    TRACKS_PAGE_SIZE,
    (offset, limit) =>
      libraryTracks(trackQuery({ sort: 'TITLE_ASC', offset, limit, folder: path() })),
    'треки папки',
  )

  createEffect(() => {
    path()
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
        title={folderName(path())}
        subtitle={path()}
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

      <div class="flex shrink-0 flex-col gap-2 px-4 pb-3">
        <span class="text-xs text-muted">{formatPlural(list.total(), TRACK_FORMS)}</span>
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
            icon={<FolderOpen aria-hidden="true" />}
            title="В этой папке нет треков"
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
