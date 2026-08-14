import { useNavigate } from '@solidjs/router'
import { MicVocal } from 'lucide-solid'
import { createEffect, For, Match, Show, Switch } from 'solid-js'

import type { Artist } from '@/shared/ipc'
import { formatPlural } from '@/shared/lib'
import { CoverArt, Spinner } from '@/shared/ui'
import {
  ALBUM_FORMS,
  artistsList,
  isNearBottom,
  libraryVersion,
  TRACK_FORMS,
} from '../model/library-store'
import { EmptyLibraryState, ErrorState, GridSkeleton, GRID_COLUMNS } from './LibraryStates'

/** Исполнители сеткой круглых обложек. */
export function ArtistsTab() {
  const navigate = useNavigate()


  createEffect(() => {
    libraryVersion()
    artistsList.load()
  })

  const open = (artist: Artist): void => {
    navigate(`/library/artist/${String(artist.id)}`)
  }

  const subtitle = (artist: Artist): string =>
    `${formatPlural(artist.albumCount, ALBUM_FORMS)} · ${formatPlural(artist.trackCount, TRACK_FORMS)}`

  return (
    <Switch>
      <Match when={artistsList.items().length === 0 && artistsList.error() !== null}>
        <ErrorState
          message={artistsList.error() ?? ''}
          onRetry={() => {
            artistsList.retry()
          }}
        />
      </Match>

      <Match when={!artistsList.loaded()}>
        <GridSkeleton columns={GRID_COLUMNS} round />
      </Match>

      <Match when={artistsList.items().length === 0}>
        <EmptyLibraryState
          icon={<MicVocal aria-hidden="true" />}
          title="Исполнителей пока нет"
          description="Добавьте папку с музыкой в настройках"
        />
      </Match>

      <Match when={artistsList.items().length > 0}>
        <div
          class="h-full min-h-0 overflow-y-auto overscroll-contain scrollbar-none pb-player-stack"
          onScroll={(event) => {
            if (isNearBottom(event.currentTarget)) artistsList.load()
          }}
        >
          <div class={`grid gap-3 px-4 pt-2 ${GRID_COLUMNS}`}>
            <For each={artistsList.items()}>
              {(artist) => (
                <button
                  type="button"
                  class="flex flex-col items-center gap-1.5 no-highlight"
                  onClick={() => {
                    open(artist)
                  }}
                >
                  <CoverArt
                    coverKey={artist.coverKey}
                    size="full"
                    rounded="full"
                    seed={artist.name}
                    alt={artist.name}
                  />
                  <span class="w-full truncate text-center text-sm font-medium text-foreground">
                    {artist.name}
                  </span>
                  <span class="w-full truncate text-center text-xs text-muted">
                    {subtitle(artist)}
                  </span>
                </button>
              )}
            </For>
          </div>

          <Show when={artistsList.loading()}>
            <div class="flex justify-center py-4">
              <Spinner size="sm" color="accent" />
            </div>
          </Show>
        </div>
      </Match>
    </Switch>
  )
}
