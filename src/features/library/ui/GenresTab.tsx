import { useNavigate } from '@solidjs/router'
import { ChevronRight, Tags } from 'lucide-solid'
import { createEffect, For, Match, Show, Switch } from 'solid-js'

import type { Genre } from '@/shared/ipc'
import { formatPlural } from '@/shared/lib'
import { Separator } from '@/shared/ui'
import {
  encodeRouteParam,
  genresList,
  libraryVersion,
  TRACK_FORMS,
} from '../model/library-store'
import { EmptyLibraryState, ErrorState, ListSkeleton } from './LibraryStates'

/** Жанры списком с количеством треков. */
export function GenresTab() {
  const navigate = useNavigate()

  createEffect(() => {
    libraryVersion()
    genresList.ensure()
  })

  const open = (genre: Genre): void => {
    navigate(`/library/genre/${encodeRouteParam(genre.name)}`)
  }

  return (
    <Switch>
      <Match when={genresList.error() !== null}>
        <ErrorState
          message={genresList.error() ?? ''}
          onRetry={() => {
            genresList.retry()
          }}
        />
      </Match>

      <Match when={genresList.data() === null}>
        <ListSkeleton rows={6} />
      </Match>

      <Match when={genresList.data()?.length === 0}>
        <EmptyLibraryState
          icon={<Tags aria-hidden="true" />}
          title="Жанров пока нет"
          description="Добавьте папку с музыкой в настройках"
        />
      </Match>

      <Match when={genresList.data()}>
        {(genres) => (
          <div class="h-full min-h-0 overflow-y-auto overscroll-contain scrollbar-none pb-player-stack">
            <For each={genres()}>
              {(genre, index) => (
                <>
                  <Show when={index() > 0}>
                    <Separator class="ms-4" />
                  </Show>
                  <button
                    type="button"
                    class="flex min-h-14 w-full items-center gap-3 px-4 text-start no-highlight"
                    onClick={() => {
                      open(genre)
                    }}
                  >
                    <span class="min-w-0 flex-1 truncate text-sm font-medium text-foreground">
                      {genre.name}
                    </span>
                    <span class="shrink-0 text-xs text-muted">
                      {formatPlural(genre.trackCount, TRACK_FORMS)}
                    </span>
                    <ChevronRight size={18} class="shrink-0 text-muted" aria-hidden="true" />
                  </button>
                </>
              )}
            </For>
          </div>
        )}
      </Match>
    </Switch>
  )
}
