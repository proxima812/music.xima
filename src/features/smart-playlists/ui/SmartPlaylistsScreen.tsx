import { useNavigate } from '@solidjs/router'
import { ChevronLeft, Pencil, Play, Plus, Sparkles, Trash2 } from 'lucide-solid'
import { createResource, createSignal, For, onCleanup, onMount, Show } from 'solid-js'

import { usePlayer } from '@/features/player/model/player-store'
import {
  libraryArtist,
  smartPlaylistDelete,
  smartPlaylistList,
  smartPlaylistResolve,
  type SmartPlaylist,
} from '@/shared/ipc'
import { settled } from '@/shared/lib'
import {
  Button,
  ConfirmDialog,
  EmptyState,
  IconButton,
  Menu,
  Screen,
  Skeleton,
  Spinner,
  toast,
  TopBar,
  type MenuAction,
} from '@/shared/ui'
import { describeRules, formatTrackCount, type RuleNames } from '../model/rules'

/** Список умных плейлистов: правила, количество треков, запуск и редактирование. */
export function SmartPlaylistsScreen() {
  const navigate = useNavigate()

  const [playlists, { refetch }] = createResource(() => smartPlaylistList())
  const [pendingDelete, setPendingDelete] = createSignal<SmartPlaylist | null>(null)

  // Ресурсы читаются через зеркала: прямое чтение поднимает общий `<Suspense>`,
  // а тот на время загрузки вынимает из DOM весь экран (docs/BUGS.md, B8).
  const items = settled(playlists)

  // Правило ARTIST_IS хранит только id — имена подтягиваем, чтобы описание читалось.
  const artistIds = (): number[] => {
    const ids = new Set<number>()
    for (const playlist of items() ?? []) {
      for (const rule of playlist.rules) {
        if (rule.kind === 'ARTIST_IS' && rule.artistId > 0) ids.add(rule.artistId)
      }
    }
    return [...ids]
  }

  const [artistNames] = createResource(
    () => (artistIds().length > 0 ? artistIds().join(',') : undefined),
    async (key: string) => {
      const ids = key.split(',').map((value) => Number.parseInt(value, 10))
      const entries = await Promise.all(
        ids.map(async (id): Promise<readonly [number, string] | null> => {
          try {
            const artist = await libraryArtist(id)
            return [id, artist.name] as const
          } catch (error: unknown) {
            console.error(`[smart] не удалось прочитать исполнителя ${String(id)}`, error)
            return null
          }
        }),
      )
      return new Map(entries.filter((entry): entry is readonly [number, string] => entry !== null))
    },
  )

  const settledNames = settled(artistNames)

  const names = (): RuleNames => {
    const map = settledNames()
    return map === undefined ? {} : { artistNames: map }
  }

  const remove = (playlist: SmartPlaylist): Promise<void> =>
    smartPlaylistDelete(playlist.id)
      .then(() => {
        toast({ title: `Плейлист «${playlist.name}» удалён` })
        void refetch()
      })
      .catch((error: unknown) => {
        console.error('[smart] не удалось удалить плейлист', error)
        toast({ title: 'Не удалось удалить плейлист', variant: 'danger' })
      })

  return (
    <Screen>
      <TopBar
        title="Умные плейлисты"
        left={
          <IconButton
            label="Назад"
            onClick={() => {
              navigate(-1)
            }}
          >
            <ChevronLeft aria-hidden="true" />
          </IconButton>
        }
        right={
          <IconButton
            label="Создать умный плейлист"
            variant="primary"
            onClick={() => {
              navigate('/smart/new')
            }}
          >
            <Plus aria-hidden="true" />
          </IconButton>
        }
      />

      <Show
        when={!playlists.loading}
        fallback={
          <div class="flex justify-center py-10">
            <Spinner />
          </div>
        }
      >
        <Show
          when={(items() ?? []).length > 0}
          fallback={
            <EmptyState
              class="min-h-[50vh]"
              icon={<Sparkles aria-hidden="true" />}
              title="Умных плейлистов пока нет"
              description="Соберите подборку из правил: например всё, что не слушали полгода."
              action={
                <Button
                  variant="primary"
                  onClick={() => {
                    navigate('/smart/new')
                  }}
                >
                  Создать плейлист
                </Button>
              }
            />
          }
        >
          <div class="flex flex-col gap-2 px-4 pt-2 pb-6">
            <For each={items() ?? []}>
              {(playlist) => (
                <SmartPlaylistRow
                  playlist={playlist}
                  names={names()}
                  onEdit={() => {
                    navigate(`/smart/${String(playlist.id)}/edit`)
                  }}
                  onDelete={() => {
                    setPendingDelete(playlist)
                  }}
                />
              )}
            </For>
          </div>
        </Show>
      </Show>

      <ConfirmDialog
        open={pendingDelete() !== null}
        onOpenChange={(open) => {
          if (!open) setPendingDelete(null)
        }}
        title="Удалить плейлист?"
        description={`«${pendingDelete()?.name ?? ''}» исчезнет из списка. Треки останутся в библиотеке.`}
        confirmLabel="Удалить"
        danger
        onConfirm={() => {
          const playlist = pendingDelete()
          return playlist === null ? undefined : remove(playlist)
        }}
      />
    </Screen>
  )
}

type SmartPlaylistRowProps = {
  playlist: SmartPlaylist
  names: RuleNames
  onEdit: () => void
  onDelete: () => void
}

/**
 * Строка списка. Количество треков считается лениво: `smart_playlist_resolve`
 * гоняет полный запрос, поэтому дёргаем его только когда строка видна.
 */
function SmartPlaylistRow(props: SmartPlaylistRowProps) {
  const player = usePlayer()
  const [visible, setVisible] = createSignal(false)
  const [starting, setStarting] = createSignal(false)

  let element: HTMLDivElement | undefined

  onMount(() => {
    const node = element
    if (node === undefined || typeof IntersectionObserver === 'undefined') {
      setVisible(true)
      return
    }

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setVisible(true)
          observer.disconnect()
        }
      },
      { rootMargin: '200px' },
    )
    observer.observe(node)

    onCleanup(() => {
      observer.disconnect()
    })
  })

  const [tracks] = createResource(
    () => (visible() ? props.playlist.id : undefined),
    (id: number) => smartPlaylistResolve(id),
  )

  const startPlayback = (ids: number[]): void => {
    if (ids.length === 0) {
      toast({ title: 'Под правила не подошёл ни один трек' })
      return
    }
    player.playTracks(ids, 0)
  }

  const play = (): void => {
    const loaded = tracks()
    if (loaded !== undefined) {
      startPlayback(loaded.map((track) => track.id))
      return
    }

    if (starting()) return
    setStarting(true)
    smartPlaylistResolve(props.playlist.id)
      .then((resolved) => {
        startPlayback(resolved.map((track) => track.id))
      })
      .catch((error: unknown) => {
        console.error('[smart] не удалось собрать плейлист', error)
        toast({ title: 'Не удалось собрать плейлист', variant: 'danger' })
      })
      .finally(() => {
        setStarting(false)
      })
  }

  const menuItems = (): MenuAction[] => [
    { label: 'Слушать', icon: <Play size={18} aria-hidden="true" />, onSelect: play },
    {
      label: 'Редактировать',
      icon: <Pencil size={18} aria-hidden="true" />,
      onSelect: props.onEdit,
    },
    {
      label: 'Удалить',
      icon: <Trash2 size={18} aria-hidden="true" />,
      danger: true,
      onSelect: props.onDelete,
    },
  ]

  return (
    <div ref={element} class="depth-raised flex items-center gap-1 rounded-2xl p-2 pr-1">
      <button
        type="button"
        class="flex min-h-14 min-w-0 flex-1 items-center gap-3 text-start no-highlight"
        onClick={play}
      >
        <span class="flex size-11 shrink-0 items-center justify-center rounded-xl bg-accent-soft text-accent-soft-foreground">
          <Sparkles size={20} aria-hidden="true" />
        </span>

        <span class="flex min-w-0 flex-1 flex-col gap-1">
          <span class="truncate text-sm font-medium text-foreground">{props.playlist.name}</span>
          <span class="flex min-w-0 items-center gap-1.5 text-xs text-muted">
            {/*
              Только осевший ресурс. Чтение в загрузке поднимает общий
              `<Suspense>` из `App.tsx`, а тот вынимает из DOM весь экран:
              строки считают треки по мере появления в окне прокрутки, и
              список дёргался бы на каждой (docs/BUGS.md, B8).
            */}
            <Show
              when={tracks.loading ? undefined : tracks()}
              fallback={<Skeleton class="h-3 w-14 shrink-0 rounded-full" />}
            >
              {(items) => (
                <span class="shrink-0 tabular-nums">{formatTrackCount(items().length)}</span>
              )}
            </Show>
            <span aria-hidden="true" class="shrink-0 text-muted">
              ·
            </span>
            <span class="truncate">
              {describeRules(props.playlist.rules, props.playlist.matchAll, props.names)}
            </span>
          </span>
        </span>

        <Show when={starting()}>
          <Spinner size="sm" />
        </Show>
      </button>

      <Menu label={`Действия: ${props.playlist.name}`} items={menuItems()} />
    </div>
  )
}
