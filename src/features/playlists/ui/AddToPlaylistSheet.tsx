import { ListMusic, Plus } from 'lucide-solid'
import { createEffect, createSignal, For, on, Show } from 'solid-js'

import { formatPlural } from '@/shared/lib'
import { EmptyState, Sheet, Spinner } from '@/shared/ui'
import {
  addTracksToPlaylist,
  allPlaylists,
  ensurePlaylists,
  playlistsLoading,
} from '../model/playlists-store'
import { CreatePlaylistDialog } from './CreatePlaylistDialog'
import { PlaylistCover } from './PlaylistCover'

const TRACK_FORMS: readonly [string, string, string] = ['трек', 'трека', 'треков']

export type AddToPlaylistSheetProps = {
  trackIds: readonly number[]
  open: boolean
  onClose: () => void
}

/** Боттом-шит «Добавить в плейлист»: список плейлистов плюс создание нового. */
export function AddToPlaylistSheet(props: AddToPlaylistSheetProps) {
  const [creating, setCreating] = createSignal(false)
  const [pendingId, setPendingId] = createSignal<number | null>(null)

  createEffect(
    on(
      () => props.open,
      (open) => {
        if (!open) {
          setPendingId(null)
          return
        }
        void ensurePlaylists()
      },
    ),
  )

  const add = (id: number): void => {
    if (pendingId() !== null) return

    setPendingId(id)
    void addTracksToPlaylist(id, props.trackIds).then((ok) => {
      setPendingId(null)
      if (ok) props.onClose()
    })
  }

  const title = (): string =>
    props.trackIds.length > 1
      ? `Добавить ${formatPlural(props.trackIds.length, TRACK_FORMS)}`
      : 'Добавить в плейлист'

  return (
    <>
      <Sheet
        open={props.open}
        onOpenChange={(open) => {
          if (!open) props.onClose()
        }}
        title={title()}
      >
        <div class="flex flex-col gap-2">
          <button
            type="button"
            class="flex min-h-14 w-full items-center gap-3 rounded-xl px-2 text-start no-highlight"
            onClick={() => setCreating(true)}
          >
            <span class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-accent-soft-foreground">
              <Plus size={20} aria-hidden="true" />
            </span>
            <span class="min-w-0 flex-1 truncate text-sm font-medium text-foreground">
              Создать новый
            </span>
          </button>

          <Show
            when={!playlistsLoading() || allPlaylists().length > 0}
            fallback={
              <div class="flex justify-center py-8">
                <Spinner />
              </div>
            }
          >
            <Show
              when={allPlaylists().length > 0}
              fallback={
                <EmptyState
                  icon={<ListMusic aria-hidden="true" />}
                  title="Плейлистов пока нет"
                  description="Создайте первый — треки попадут сразу в него."
                />
              }
            >
              <div class="flex flex-col">
                <For each={allPlaylists()}>
                  {(playlist) => (
                    <button
                      type="button"
                      class="flex min-h-14 w-full items-center gap-3 rounded-xl px-2 text-start no-highlight"
                      disabled={pendingId() !== null}
                      onClick={() => add(playlist.id)}
                    >
                      <PlaylistCover
                        playlistId={playlist.id}
                        coverKey={playlist.coverKey}
                        trackCount={playlist.trackCount}
                        name={playlist.name}
                        size={40}
                      />
                      <span class="flex min-w-0 flex-1 flex-col gap-0.5">
                        <span class="truncate text-sm font-medium text-foreground">
                          {playlist.name}
                        </span>
                        <span class="truncate text-xs text-muted">
                          {formatPlural(playlist.trackCount, TRACK_FORMS)}
                        </span>
                      </span>
                      <Show when={pendingId() === playlist.id}>
                        <Spinner size="sm" color="current" />
                      </Show>
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </Show>
        </div>
      </Sheet>

      <CreatePlaylistDialog
        open={creating()}
        onOpenChange={setCreating}
        trackIds={props.trackIds}
        onCreated={() => {
          props.onClose()
        }}
      />
    </>
  )
}
