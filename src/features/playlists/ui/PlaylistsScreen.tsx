import { useNavigate } from '@solidjs/router'
import { ChevronRight, ListMusic, Plus, Sparkles, TriangleAlert } from 'lucide-solid'
import { createSignal, For, onMount, Show } from 'solid-js'

import type { SmartPlaylist } from '@/shared/ipc'
import { formatDurationHuman, formatPlural } from '@/shared/lib'
import {
  Button,
  EmptyState,
  IconButton,
  Screen,
  SectionHeader,
  Spinner,
  TopBar,
} from '@/shared/ui'
import {
  allPlaylists,
  allSmartPlaylists,
  playlistsError,
  playlistsLoading,
  refreshPlaylists,
} from '../model/playlists-store'
import { CreatePlaylistDialog } from './CreatePlaylistDialog'
import { PlaylistCover } from './PlaylistCover'

const TRACK_FORMS: readonly [string, string, string] = ['трек', 'трека', 'треков']

const RULE_FORMS: readonly [string, string, string] = ['правило', 'правила', 'правил']

/** Экран плейлистов: умные сверху, свои снизу. */
export function PlaylistsScreen() {
  const navigate = useNavigate()
  const [creating, setCreating] = createSignal(false)

  onMount(() => {
    void refreshPlaylists()
  })

  const isEmpty = (): boolean => allPlaylists().length === 0 && allSmartPlaylists().length === 0

  const isInitialLoad = (): boolean => playlistsLoading() && isEmpty()

  return (
    <Screen>
      <TopBar
        title="Плейлисты"
        right={
          <IconButton label="Создать плейлист" onClick={() => setCreating(true)}>
            <Plus aria-hidden="true" />
          </IconButton>
        }
      />

      <Show when={isInitialLoad()}>
        <div class="flex justify-center py-12">
          <Spinner size="lg" color="accent" />
        </div>
      </Show>

      <Show when={playlistsError()}>
        {(message) => (
          <EmptyState
            icon={<TriangleAlert aria-hidden="true" />}
            title="Не удалось загрузить плейлисты"
            description={message()}
            action={
              <Button
                variant="secondary"
                onClick={() => {
                  void refreshPlaylists()
                }}
              >
                Повторить
              </Button>
            }
          />
        )}
      </Show>

      <Show when={!isInitialLoad() && playlistsError() === null}>
        <div class="flex flex-col pb-6">
          <section>
            <SectionHeader
              title="Умные плейлисты"
              description="Собираются правилами, обновляются сами"
              action={
                <Button variant="ghost" size="sm" onClick={() => navigate('/smart')}>
                  Все
                </Button>
              }
            />

            <div class="flex flex-col">
              <For each={allSmartPlaylists()}>
                {(playlist) => (
                  <button
                    type="button"
                    class="flex min-h-16 w-full items-center gap-3 px-4 py-2 text-start no-highlight"
                    onClick={() => navigate(`/smart/${String(playlist.id)}/edit`)}
                  >
                    <span class="flex size-14 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-accent-soft-foreground">
                      <Sparkles size={22} aria-hidden="true" />
                    </span>
                    <span class="flex min-w-0 flex-1 flex-col gap-0.5">
                      <span class="truncate text-sm font-medium text-foreground">
                        {playlist.name}
                      </span>
                      <span class="truncate text-xs text-muted">{smartSubtitle(playlist)}</span>
                    </span>
                    <ChevronRight size={18} class="shrink-0 text-muted" aria-hidden="true" />
                  </button>
                )}
              </For>

              <button
                type="button"
                class="flex min-h-14 w-full items-center gap-3 px-4 py-2 text-start no-highlight"
                onClick={() => navigate('/smart/new')}
              >
                <span class="flex size-14 shrink-0 items-center justify-center rounded-xl border border-dashed border-border text-muted">
                  <Plus size={20} aria-hidden="true" />
                </span>
                <span class="min-w-0 flex-1 truncate text-sm font-medium text-foreground">
                  Новый умный плейлист
                </span>
              </button>
            </div>
          </section>

          <section>
            <SectionHeader
              title="Мои плейлисты"
              description={
                allPlaylists().length > 0
                  ? formatPlural(allPlaylists().length, ['плейлист', 'плейлиста', 'плейлистов'])
                  : undefined
              }
              action={
                <Show when={allPlaylists().length > 0}>
                  <Button variant="ghost" size="sm" onClick={() => setCreating(true)}>
                    Создать
                  </Button>
                </Show>
              }
            />

            <Show
              when={allPlaylists().length > 0}
              fallback={
                <EmptyState
                  icon={<ListMusic aria-hidden="true" />}
                  title="Плейлистов пока нет"
                  description="Соберите свой первый список — он появится здесь."
                  action={
                    <Button variant="primary" onClick={() => setCreating(true)}>
                      Создать плейлист
                    </Button>
                  }
                />
              }
            >
              <div class="flex flex-col">
                <For each={allPlaylists()}>
                  {(playlist) => (
                    <button
                      type="button"
                      class="flex min-h-16 w-full items-center gap-3 px-4 py-2 text-start no-highlight"
                      onClick={() => navigate(`/playlists/${String(playlist.id)}`)}
                    >
                      <PlaylistCover
                        playlistId={playlist.id}
                        coverKey={playlist.coverKey}
                        trackCount={playlist.trackCount}
                        name={playlist.name}
                      />
                      <span class="flex min-w-0 flex-1 flex-col gap-0.5">
                        <span class="truncate text-sm font-medium text-foreground">
                          {playlist.name}
                        </span>
                        <span class="truncate text-xs text-muted">
                          {formatPlural(playlist.trackCount, TRACK_FORMS)}
                          {playlist.durationMs > 0
                            ? ` · ${formatDurationHuman(playlist.durationMs)}`
                            : ''}
                        </span>
                      </span>
                      <ChevronRight size={18} class="shrink-0 text-muted" aria-hidden="true" />
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </section>
        </div>
      </Show>

      <CreatePlaylistDialog
        open={creating()}
        onOpenChange={setCreating}
        onCreated={(playlist) => {
          navigate(`/playlists/${String(playlist.id)}`)
        }}
      />
    </Screen>
  )
}

function smartSubtitle(playlist: SmartPlaylist): string {
  const parts: string[] = [formatPlural(playlist.rules.length, RULE_FORMS)]
  parts.push(playlist.matchAll ? 'все условия' : 'любое условие')
  if (playlist.limit !== null) parts.push(`не больше ${String(playlist.limit)}`)
  return parts.join(' · ')
}
