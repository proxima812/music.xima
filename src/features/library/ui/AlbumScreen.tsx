import { useNavigate, useParams } from '@solidjs/router'
import { ArrowLeft, Disc3 } from 'lucide-solid'
import { createResource, createSignal, For, Match, Show, Switch } from 'solid-js'

import { usePlayer } from '@/features/player/model/player-store'
import { TrackMenu } from '@/features/player/ui/TrackMenu'
import { libraryAlbum, libraryAlbumTracks, type Track } from '@/shared/ipc'
import { formatDurationHuman, formatPlural } from '@/shared/lib'
import { CoverArt, IconButton, Screen, Skeleton, TrackRow, TopBar } from '@/shared/ui'
import { libraryVersion, TRACK_FORMS } from '../model/library-store'
import { EmptyLibraryState, ErrorState, ListSkeleton, PlaybackButtons } from './LibraryStates'

/** Альбом: шапка с обложкой и метаданными, кнопки воспроизведения, треки с номерами. */
export function AlbumScreen() {
  const params = useParams<{ id: string }>()
  const navigate = useNavigate()
  const player = usePlayer()

  const [menuTrack, setMenuTrack] = createSignal<Track | null>(null)

  const albumId = (): number | null => {
    const parsed = Number.parseInt(params.id, 10)
    return Number.isFinite(parsed) ? parsed : null
  }

  const source = (): { id: number; version: number } | undefined => {
    const id = albumId()
    return id === null ? undefined : { id, version: libraryVersion() }
  }

  const [album, albumActions] = createResource(source, (key) => libraryAlbum(key.id))
  const [tracks, trackActions] = createResource(source, (key) => libraryAlbumTracks(key.id))

  const items = (): readonly Track[] => tracks() ?? []

  const reload = (): void => {
    void albumActions.refetch()
    void trackActions.refetch()
  }

  const playFrom = (index: number): void => {
    const list = items()
    if (list.length === 0) return
    player.setShuffle(false)
    player.playTracks(
      list.map((track) => track.id),
      index,
    )
  }

  const shuffleAll = (): void => {
    const list = items()
    if (list.length === 0) return
    player.setShuffle(true)
    player.playTracks(
      list.map((track) => track.id),
      Math.floor(Math.random() * list.length),
    )
  }

  const meta = (): string => {
    const current = album()
    if (current === undefined) return ''

    const parts: string[] = []
    if (current.year !== null) parts.push(String(current.year))
    parts.push(formatPlural(current.trackCount, TRACK_FORMS))
    if (current.durationMs > 0) parts.push(formatDurationHuman(current.durationMs))
    return parts.join(' · ')
  }

  return (
    <Screen>
      <TopBar
        title={album()?.title ?? 'Альбом'}
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

      <Switch>
        <Match when={albumId() === null}>
          <EmptyLibraryState
            icon={<Disc3 aria-hidden="true" />}
            title="Альбом не найден"
            description="Ссылка ведёт на несуществующий альбом"
            withSettings={false}
          />
        </Match>

        <Match when={album.error !== undefined || tracks.error !== undefined}>
          <ErrorState message={errorMessage(album.error ?? tracks.error)} onRetry={reload} />
        </Match>

        <Match when={album()}>
          {(current) => (
            <>
              <div class="flex flex-col items-center gap-3 px-6 pt-2 pb-4">
                <div class="w-56 max-w-[70%]">
                  <CoverArt
                    coverKey={current().coverKey}
                    size="full"
                    rounded="lg"
                    seed={`${current().title}·${current().artistName ?? ''}`}
                    alt={current().title}
                  />
                </div>

                <div class="flex w-full flex-col items-center gap-1">
                  <h2 class="line-clamp-2 text-center text-lg font-semibold text-foreground">
                    {current().title}
                  </h2>

                  <Show
                    when={current().artistId !== null}
                    fallback={
                      <span class="truncate text-sm text-muted">
                        {current().artistName ?? 'Неизвестный исполнитель'}
                      </span>
                    }
                  >
                    <button
                      type="button"
                      class="max-w-full truncate text-sm text-accent no-highlight"
                      onClick={() => {
                        navigate(`/library/artist/${String(current().artistId)}`)
                      }}
                    >
                      {current().artistName ?? 'Неизвестный исполнитель'}
                    </button>
                  </Show>

                  <span class="text-xs text-muted">{meta()}</span>
                </div>

                <PlaybackButtons
                  onPlay={() => {
                    playFrom(0)
                  }}
                  onShuffle={shuffleAll}
                  disabled={items().length === 0}
                />
              </div>

              <Show when={!tracks.loading} fallback={<ListSkeleton rows={6} />}>
                <For each={items()}>
                  {(track, index) => (
                    <div class="flex w-full items-center">
                      <span class="w-9 shrink-0 pl-4 text-end text-xs tabular-nums text-muted">
                        {track.trackNumber ?? index() + 1}
                      </span>
                      <TrackRow
                        class="min-w-0 flex-1"
                        track={track}
                        index={index()}
                        active={player.state.trackId === track.id}
                        showArtwork={false}
                        onPlay={(_track, position) => {
                          playFrom(position ?? 0)
                        }}
                        onMenu={(selected) => {
                          setMenuTrack(selected)
                        }}
                      />
                    </div>
                  )}
                </For>
              </Show>
            </>
          )}
        </Match>

        <Match when={album.loading}>
          <div class="flex flex-col items-center gap-3 px-6 pt-2 pb-4">
            <Skeleton class="aspect-square w-56 max-w-[70%] rounded-2xl" />
            <Skeleton class="h-5 w-2/3 rounded-md" />
            <Skeleton class="h-4 w-1/3 rounded-md" />
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

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message
  return typeof error === 'string' ? error : 'Неизвестная ошибка'
}
