import { useNavigate, useParams } from '@solidjs/router'
import { ArrowLeft, MicVocal } from 'lucide-solid'
import { createResource, createSignal, For, Match, Show, Switch } from 'solid-js'

import { usePlayer } from '@/features/player/model/player-store'
import { TrackMenu } from '@/features/player/ui/TrackMenu'
import {
  libraryArtist,
  libraryArtistAlbums,
  libraryArtistTracks,
  type Album,
  type Track,
} from '@/shared/ipc'
import { formatPlural, settled } from '@/shared/lib'
import {
  CoverArt,
  IconButton,
  Screen,
  SectionHeader,
  Skeleton,
  TopBar,
  TrackRow,
} from '@/shared/ui'
import { ALBUM_FORMS, libraryVersion, TRACK_FORMS } from '../model/library-store'
import {
  EmptyLibraryState,
  ErrorState,
  GRID_COLUMNS,
  ListSkeleton,
  PlaybackButtons,
} from './LibraryStates'

/** Исполнитель: шапка, альбомы сеткой и все треки. */
export function ArtistScreen() {
  const params = useParams<{ id: string }>()
  const navigate = useNavigate()
  const player = usePlayer()

  const [menuTrack, setMenuTrack] = createSignal<Track | null>(null)

  const artistId = (): number | null => {
    const parsed = Number.parseInt(params.id, 10)
    return Number.isFinite(parsed) ? parsed : null
  }

  const source = (): { id: number; version: number } | undefined => {
    const id = artistId()
    return id === null ? undefined : { id, version: libraryVersion() }
  }

  const [artist, artistActions] = createResource(source, (key) => libraryArtist(key.id))
  const [albums, albumActions] = createResource(source, (key) => libraryArtistAlbums(key.id))
  const [tracks, trackActions] = createResource(source, (key) => libraryArtistTracks(key.id))

  // Ресурсы читаются через зеркала: прямое чтение поднимает общий `<Suspense>`,
  // и `library:changed` вынимал бы открытый экран из DOM (docs/BUGS.md, B8).
  const current = settled(artist)
  const settledAlbums = settled(albums)
  const settledTracks = settled(tracks)

  const items = (): readonly Track[] => settledTracks() ?? []
  const artistAlbums = (): readonly Album[] => settledAlbums() ?? []

  const reload = (): void => {
    void artistActions.refetch()
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

  return (
    <Screen>
      <TopBar
        title={current()?.name ?? 'Исполнитель'}
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
        <Match when={artistId() === null}>
          <EmptyLibraryState
            icon={<MicVocal aria-hidden="true" />}
            title="Исполнитель не найден"
            description="Ссылка ведёт на несуществующего исполнителя"
            withSettings={false}
          />
        </Match>

        <Match
          when={
            artist.error !== undefined || albums.error !== undefined || tracks.error !== undefined
          }
        >
          <ErrorState
            message={errorMessage(artist.error ?? albums.error ?? tracks.error)}
            onRetry={reload}
          />
        </Match>

        <Match when={current()}>
          {(current) => (
            <>
              <div class="flex flex-col items-center gap-3 px-6 pt-2 pb-4">
                <div class="w-40 max-w-[55%]">
                  <CoverArt
                    coverKey={current().coverKey}
                    size="full"
                    rounded="full"
                    seed={current().name}
                    alt={current().name}
                  />
                </div>

                <div class="flex w-full flex-col items-center gap-1">
                  <h2 class="line-clamp-2 text-center text-lg font-semibold text-foreground">
                    {current().name}
                  </h2>
                  <span class="text-xs text-muted">
                    {formatPlural(current().albumCount, ALBUM_FORMS)} ·{' '}
                    {formatPlural(current().trackCount, TRACK_FORMS)}
                  </span>
                </div>

                <PlaybackButtons
                  onPlay={() => {
                    playFrom(0)
                  }}
                  onShuffle={shuffleAll}
                  disabled={items().length === 0}
                />
              </div>

              <Show when={artistAlbums().length > 0}>
                <SectionHeader title="Альбомы" />
                <div class={`grid gap-3 px-4 pb-2 ${GRID_COLUMNS}`}>
                  <For each={artistAlbums()}>
                    {(album) => (
                      <button
                        type="button"
                        class="flex flex-col gap-1.5 text-start no-highlight"
                        onClick={() => {
                          navigate(`/library/album/${String(album.id)}`)
                        }}
                      >
                        <CoverArt
                          coverKey={album.coverKey}
                          size="full"
                          rounded="md"
                          seed={album.title}
                          alt={album.title}
                        />
                        <span class="truncate text-sm font-medium text-foreground">
                          {album.title}
                        </span>
                        <span class="truncate text-xs text-muted">
                          {album.year === null ? '' : String(album.year)}
                        </span>
                      </button>
                    )}
                  </For>
                </div>
              </Show>

              <SectionHeader title="Все треки" />

              <Show when={!tracks.loading} fallback={<ListSkeleton rows={6} />}>
                <For each={items()}>
                  {(track, index) => (
                    <TrackRow
                      track={track}
                      index={index()}
                      active={player.state.trackId === track.id}
                      onPlay={(_track, position) => {
                        playFrom(position ?? 0)
                      }}
                      onMenu={(selected) => {
                        setMenuTrack(selected)
                      }}
                    />
                  )}
                </For>
              </Show>
            </>
          )}
        </Match>

        <Match when={artist.loading}>
          <div class="flex flex-col items-center gap-3 px-6 pt-2 pb-4">
            <Skeleton class="aspect-square w-40 max-w-[55%] rounded-full" />
            <Skeleton class="h-5 w-1/2 rounded-md" />
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
