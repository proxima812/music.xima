import { useNavigate } from '@solidjs/router'
import { Disc3 } from 'lucide-solid'
import { createEffect, For, Match, Show, Switch } from 'solid-js'

import type { Album } from '@/shared/ipc'
import { formatPlural } from '@/shared/lib'
import { CoverArt, Spinner } from '@/shared/ui'
import {
  albumsList,
  isNearBottom,
  libraryVersion,
  TRACK_FORMS,
} from '../model/library-store'
import { EmptyLibraryState, ErrorState, GridSkeleton, GRID_COLUMNS } from './LibraryStates'

/** Альбомы сеткой обложек; ширина сетки — общая. */
export function AlbumsTab() {
  const navigate = useNavigate()


  createEffect(() => {
    libraryVersion()
    albumsList.load()
  })

  const open = (album: Album): void => {
    navigate(`/library/album/${String(album.id)}`)
  }

  return (
    <Switch>
      <Match when={albumsList.items().length === 0 && albumsList.error() !== null}>
        <ErrorState
          message={albumsList.error() ?? ''}
          onRetry={() => {
            albumsList.retry()
          }}
        />
      </Match>

      <Match when={!albumsList.loaded()}>
        <GridSkeleton columns={GRID_COLUMNS} />
      </Match>

      <Match when={albumsList.items().length === 0}>
        <EmptyLibraryState
          icon={<Disc3 aria-hidden="true" />}
          title="Альбомов пока нет"
          description="Добавьте папку с музыкой в настройках"
        />
      </Match>

      <Match when={albumsList.items().length > 0}>
        <div
          class="h-full min-h-0 overflow-y-auto overscroll-contain scrollbar-none pb-player-stack"
          onScroll={(event) => {
            if (isNearBottom(event.currentTarget)) albumsList.load()
          }}
        >
          <div class={`grid gap-3 px-4 pt-2 ${GRID_COLUMNS}`}>
            <For each={albumsList.items()}>
              {(album) => (
                <button
                  type="button"
                  class="flex flex-col gap-1.5 text-start no-highlight"
                  onClick={() => {
                    open(album)
                  }}
                >
                  <CoverArt
                    coverKey={album.coverKey}
                    seed={`${album.title}·${album.artistName ?? ''}`}
                    size="full"
                    rounded="md"
                    alt={album.title}
                  />
                  <span class="truncate text-sm font-medium text-foreground">{album.title}</span>
                  <span class="truncate text-xs text-muted">
                    {album.artistName ?? 'Неизвестный исполнитель'}
                  </span>
                  <span class="truncate text-xs text-muted">
                    {formatPlural(album.trackCount, TRACK_FORMS)}
                  </span>
                </button>
              )}
            </For>
          </div>

          <Show when={albumsList.loading()}>
            <div class="flex justify-center py-4">
              <Spinner size="sm" color="accent" />
            </div>
          </Show>
        </div>
      </Match>
    </Switch>
  )
}
