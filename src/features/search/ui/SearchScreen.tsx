import { useNavigate } from '@solidjs/router'
import { Clock, SearchX, Search as SearchIcon } from 'lucide-solid'
import { createSignal, For, onCleanup, onMount, Show, type JSX } from 'solid-js'

import { usePlayer } from '@/features/player/model/player-store'
import type { Album, Artist, Playlist, SearchResults } from '@/shared/ipc'
import { formatCount, formatPlural } from '@/shared/lib'
import {
  Button,
  CoverArt,
  EmptyState,
  Screen,
  SearchInput,
  SectionHeader,
  Spinner,
  TrackRow,
} from '@/shared/ui'
import {
  clearSearchHistory,
  createSearchStore,
  isEmptyResults,
  loadSearchHistory,
  rememberQuery,
  searchHistory,
} from '../model/search-store'

/** Сколько элементов секции видно до нажатия «Все». */
const PREVIEW_COUNT = 5

const TRACK_FORMS: readonly [string, string, string] = ['трек', 'трека', 'треков']

const ALBUM_FORMS: readonly [string, string, string] = ['альбом', 'альбома', 'альбомов']

type SectionKey = 'tracks' | 'artists' | 'albums' | 'playlists'

/** Поиск по библиотеке: треки, исполнители, альбомы и плейлисты в одном списке. */
export function SearchScreen() {
  const navigate = useNavigate()
  const player = usePlayer()
  const store = createSearchStore()

  const [expanded, setExpanded] = createSignal<readonly SectionKey[]>([])

  onMount(() => {
    void loadSearchHistory()
  })

  onCleanup(() => {
    store.dispose()
  })

  const isExpanded = (key: SectionKey): boolean => expanded().includes(key)

  const expand = (key: SectionKey): void => {
    setExpanded((previous) => (previous.includes(key) ? previous : [...previous, key]))
  }

  const setQuery = (value: string): void => {
    setExpanded([])
    store.setQuery(value)
  }

  const repeatQuery = (value: string): void => {
    setExpanded([])
    store.submit(value)
  }

  const playFound = (results: SearchResults, index: number): void => {
    rememberQuery(store.query())
    player.playTracks(
      results.tracks.map((track) => track.id),
      index,
    )
  }

  const openArtist = (artist: Artist): void => {
    rememberQuery(store.query())
    navigate(`/library/artist/${String(artist.id)}`)
  }

  const openAlbum = (album: Album): void => {
    rememberQuery(store.query())
    navigate(`/library/album/${String(album.id)}`)
  }

  const openPlaylist = (playlist: Playlist): void => {
    rememberQuery(store.query())
    navigate(`/playlists/${String(playlist.id)}`)
  }

  return (
    <Screen>
      <div class="sticky top-0 z-20 bg-background/90 px-4 py-2 backdrop-blur-md">
        <SearchInput
          value={store.query()}
          onChange={setQuery}
          onSubmit={repeatQuery}
          autofocus
          placeholder="Треки, исполнители, альбомы"
        />
      </div>

      <Show when={store.error()}>
        {(message) => (
          <EmptyState
            icon={<SearchX aria-hidden="true" />}
            title="Поиск не сработал"
            description={message()}
            action={
              <Button variant="secondary" onClick={() => repeatQuery(store.query())}>
                Повторить
              </Button>
            }
          />
        )}
      </Show>

      <Show when={store.query().trim() === '' && store.error() === null}>
        <HistoryPanel onPick={repeatQuery} />
      </Show>

      <Show when={store.loading() && store.results() === null}>
        <div class="flex justify-center py-10">
          <Spinner size="lg" color="accent" />
        </div>
      </Show>

      <Show when={store.results()}>
        {(results) => (
          <Show
            when={!isEmptyResults(results())}
            fallback={
              <EmptyState
                icon={<SearchX aria-hidden="true" />}
                title="Ничего не найдено"
                description={`По запросу «${store.query().trim()}» в библиотеке пусто.`}
              />
            }
          >
            <div class="flex flex-col pb-6">
              <Section
                title="Треки"
                total={results().tracks.length}
                expanded={isExpanded('tracks')}
                onExpand={() => expand('tracks')}
              >
                <For each={visible(results().tracks, isExpanded('tracks'))}>
                  {(track, index) => (
                    <TrackRow
                      track={track}
                      index={index()}
                      active={player.current?.id === track.id}
                      onPlay={() => playFound(results(), index())}
                    />
                  )}
                </For>
              </Section>

              <Section
                title="Исполнители"
                total={results().artists.length}
                expanded={isExpanded('artists')}
                onExpand={() => expand('artists')}
              >
                <For each={visible(results().artists, isExpanded('artists'))}>
                  {(artist) => (
                    <ResultRow
                      title={artist.name}
                      subtitle={`${formatPlural(artist.albumCount, ALBUM_FORMS)} · ${formatPlural(
                        artist.trackCount,
                        TRACK_FORMS,
                      )}`}
                      onClick={() => openArtist(artist)}
                      leading={
                        <CoverArt
                          coverKey={artist.coverKey}
                          size="sm"
                          rounded="full"
                          seed={artist.name}
                          alt={artist.name}
                        />
                      }
                    />
                  )}
                </For>
              </Section>

              <Section
                title="Альбомы"
                total={results().albums.length}
                expanded={isExpanded('albums')}
                onExpand={() => expand('albums')}
              >
                <For each={visible(results().albums, isExpanded('albums'))}>
                  {(album) => (
                    <ResultRow
                      title={album.title}
                      subtitle={albumSubtitle(album)}
                      onClick={() => openAlbum(album)}
                      leading={
                        <CoverArt
                          coverKey={album.coverKey}
                          seed={album.title}
                          size="sm"
                          rounded="md"
                          alt={album.title}
                        />
                      }
                    />
                  )}
                </For>
              </Section>

              <Section
                title="Плейлисты"
                total={results().playlists.length}
                expanded={isExpanded('playlists')}
                onExpand={() => expand('playlists')}
              >
                <For each={visible(results().playlists, isExpanded('playlists'))}>
                  {(playlist) => (
                    <ResultRow
                      title={playlist.name}
                      subtitle={formatPlural(playlist.trackCount, TRACK_FORMS)}
                      onClick={() => openPlaylist(playlist)}
                      leading={
                        <CoverArt
                          coverKey={playlist.coverKey}
                          size="sm"
                          rounded="md"
                          seed={playlist.name}
                          alt={playlist.name}
                        />
                      }
                    />
                  )}
                </For>
              </Section>
            </div>
          </Show>
        )}
      </Show>
    </Screen>
  )
}

function visible<T>(items: readonly T[], expanded: boolean): readonly T[] {
  return expanded ? items : items.slice(0, PREVIEW_COUNT)
}

function albumSubtitle(album: Album): string {
  const parts: string[] = []
  if (album.artistName !== null && album.artistName !== '') parts.push(album.artistName)
  if (album.year !== null) parts.push(String(album.year))
  parts.push(formatPlural(album.trackCount, TRACK_FORMS))
  return parts.join(' · ')
}

/** Секция результатов: заголовок, кнопка «Все» и сами строки. */
function Section(props: {
  title: string
  total: number
  expanded: boolean
  onExpand: () => void
  children: JSX.Element
}) {
  return (
    <Show when={props.total > 0}>
      <section>
        <SectionHeader
          title={props.title}
          description={formatCount(props.total)}
          action={
            <Show when={!props.expanded && props.total > PREVIEW_COUNT}>
              <Button variant="ghost" size="sm" onClick={() => props.onExpand()}>
                Все
              </Button>
            </Show>
          }
        />
        {props.children}
      </section>
    </Show>
  )
}

function ResultRow(props: {
  leading: JSX.Element
  title: string
  subtitle: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      class="flex min-h-16 w-full items-center gap-3 px-4 py-2 text-start no-highlight"
      onClick={() => props.onClick()}
    >
      {props.leading}
      <span class="flex min-w-0 flex-1 flex-col gap-0.5">
        <span class="truncate text-sm font-medium text-foreground">{props.title}</span>
        <span class="truncate text-xs text-muted">{props.subtitle}</span>
      </span>
    </button>
  )
}

/** Экран до ввода: недавние запросы либо подсказка. */
function HistoryPanel(props: { onPick: (query: string) => void }) {
  return (
    <Show
      when={searchHistory().length > 0}
      fallback={
        <EmptyState
          icon={<SearchIcon aria-hidden="true" />}
          title="Что ищем?"
          description="Название трека, исполнитель, альбом или плейлист — всё сразу."
        />
      }
    >
      <section>
        <SectionHeader
          title="Недавние запросы"
          action={
            <Button variant="ghost" size="sm" onClick={() => clearSearchHistory()}>
              Очистить
            </Button>
          }
        />

        <div class="flex flex-col">
          <For each={searchHistory()}>
            {(query) => (
              <button
                type="button"
                class="flex min-h-11 w-full items-center gap-3 px-4 py-2 text-start no-highlight"
                onClick={() => props.onPick(query)}
              >
                <Clock size={18} class="shrink-0 text-muted" aria-hidden="true" />
                <span class="min-w-0 flex-1 truncate text-sm text-foreground">{query}</span>
              </button>
            )}
          </For>
        </div>
      </section>
    </Show>
  )
}
