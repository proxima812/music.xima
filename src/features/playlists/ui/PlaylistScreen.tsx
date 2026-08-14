import { useNavigate, useParams } from '@solidjs/router'
import {
  ChevronLeft,
  GripVertical,
  ListMusic,
  Pencil,
  Play,
  Shuffle,
  Trash2,
  TriangleAlert,
} from 'lucide-solid'
import { createResource, createSignal, For, Show, type JSX, type Resource } from 'solid-js'

import { usePlayer } from '@/features/player/model/player-store'
import { playlistGet, playlistTracks, type Playlist, type Track } from '@/shared/ipc'
import { formatDurationHuman, formatPlural } from '@/shared/lib'
import {
  Button,
  ConfirmDialog,
  EmptyState,
  IconButton,
  Menu,
  Screen,
  Spinner,
  TopBar,
  TRACK_ROW_HEIGHT,
  TrackRow,
  type MenuAction,
} from '@/shared/ui'
import {
  deletePlaylist,
  moveItem,
  removeTrackAt,
  reorderPlaylist,
} from '../model/playlists-store'
import { PlaylistCover } from './PlaylistCover'
import { RenamePlaylistDialog } from './RenamePlaylistDialog'

const TRACK_FORMS: readonly [string, string, string] = ['трек', 'трека', 'треков']

/** Порог свайпа, после которого трек убирается из плейлиста. */
const SWIPE_REMOVE_PX = 96

/** Сдвиг, после которого жест считается горизонтальным, а не скроллом. */
const SWIPE_SLOP_PX = 10

/** Экран плейлиста: шапка, воспроизведение, перетаскивание и свайп-удаление. */
export function PlaylistScreen() {
  const params = useParams<{ id: string }>()
  const navigate = useNavigate()
  const player = usePlayer()

  const playlistId = (): number | undefined => {
    const parsed = Number.parseInt(params.id, 10)
    return Number.isFinite(parsed) ? parsed : undefined
  }

  const [playlist, { refetch: refetchPlaylist }] = createResource(playlistId, (id: number) =>
    playlistGet(id),
  )
  const [tracks, { mutate: mutateTracks, refetch: refetchTracks }] = createResource(
    playlistId,
    (id: number) => playlistTracks(id),
  )

  const [renaming, setRenaming] = createSignal(false)
  const [deleting, setDeleting] = createSignal(false)
  const [dragIndex, setDragIndex] = createSignal<number | null>(null)
  const [dragOffset, setDragOffset] = createSignal(0)

  let dragStartY = 0

  const items = (): readonly Track[] => tracks() ?? []

  const trackIds = (): number[] => items().map((track) => track.id)

  const targetIndex = (): number | null => {
    const from = dragIndex()
    if (from === null) return null

    const shift = Math.round(dragOffset() / TRACK_ROW_HEIGHT)
    const last = Math.max(0, items().length - 1)
    return Math.min(Math.max(from + shift, 0), last)
  }

  /** На сколько сдвинуть чужую строку, пока перетаскивают соседнюю. */
  const shiftFor = (index: number): number => {
    const from = dragIndex()
    const to = targetIndex()
    if (from === null || to === null || index === from) return 0

    if (from < to && index > from && index <= to) return -TRACK_ROW_HEIGHT
    if (from > to && index >= to && index < from) return TRACK_ROW_HEIGHT
    return 0
  }

  const startDrag = (index: number, clientY: number): void => {
    dragStartY = clientY
    setDragIndex(index)
    setDragOffset(0)
  }

  const moveDrag = (clientY: number): void => {
    if (dragIndex() === null) return
    setDragOffset(clientY - dragStartY)
  }

  const endDrag = (): void => {
    const from = dragIndex()
    const to = targetIndex()
    setDragIndex(null)
    setDragOffset(0)

    if (from === null || to === null || from === to) return

    const id = playlistId()
    if (id === undefined) return

    const previous = items()
    mutateTracks(moveItem(previous, from, to))

    void reorderPlaylist(id, from, to).then((ok) => {
      if (ok) {
        void refetchPlaylist()
        return
      }
      mutateTracks([...previous])
    })
  }

  const remove = (index: number): void => {
    const id = playlistId()
    if (id === undefined) return

    const previous = items()
    const removed = previous[index]
    if (removed === undefined) return

    mutateTracks(previous.filter((_, position) => position !== index))

    void removeTrackAt(id, index).then((ok) => {
      if (ok) {
        void refetchPlaylist()
        return
      }
      mutateTracks([...previous])
    })
  }

  const play = (index: number): void => {
    const list = trackIds()
    if (list.length === 0) return
    player.setShuffle(false)
    player.playTracks(list, index)
  }

  const shuffle = (): void => {
    const list = trackIds()
    if (list.length === 0) return
    player.setShuffle(true)
    player.playTracks(list, Math.floor(Math.random() * list.length))
  }

  const menuItems = (): readonly MenuAction[] => [
    {
      label: 'Переименовать',
      icon: <Pencil size={18} aria-hidden="true" />,
      onSelect: () => setRenaming(true),
    },
    {
      label: 'Удалить плейлист',
      icon: <Trash2 size={18} aria-hidden="true" />,
      danger: true,
      onSelect: () => setDeleting(true),
    },
  ]

  return (
    <Screen>
      <TopBar
        title={playlist()?.name ?? 'Плейлист'}
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
          <Show when={playlist() !== undefined}>
            <Menu items={menuItems()} label="Меню плейлиста" />
          </Show>
        }
      />

      <Show
        when={playlist.error === undefined}
        fallback={
          <EmptyState
            icon={<TriangleAlert aria-hidden="true" />}
            title="Плейлист не открылся"
            description="Возможно, он уже удалён."
            action={
              <Button variant="secondary" onClick={() => navigate('/playlists', { replace: true })}>
                К плейлистам
              </Button>
            }
          />
        }
      >
        <Show
          when={playlist()}
          fallback={
            <div class="flex justify-center py-12">
              <Spinner size="lg" color="accent" />
            </div>
          }
        >
          {(current) => (
            <>
              <Header
                playlist={current()}
                tracks={tracks}
                onPlay={() => play(0)}
                onShuffle={shuffle}
              />

              <Show
                when={items().length > 0}
                fallback={
                  <Show when={!tracks.loading}>
                    <EmptyState
                      icon={<ListMusic aria-hidden="true" />}
                      title="Плейлист пуст"
                      description="Добавьте треки через меню трека — «Добавить в плейлист»."
                    />
                  </Show>
                }
              >
                <p class="px-4 pb-1 text-xs text-muted">
                  Свайп влево убирает трек, ручка справа меняет порядок
                </p>

                <div class="flex flex-col">
                  <For each={items()}>
                    {(track, index) => (
                      <ReorderableRow
                        track={track}
                        index={index()}
                        active={player.current?.id === track.id}
                        dragging={dragIndex() === index()}
                        shift={shiftFor(index())}
                        dragOffset={dragIndex() === index() ? dragOffset() : 0}
                        onPlay={() => play(index())}
                        onRemove={() => remove(index())}
                        onDragStart={(clientY) => startDrag(index(), clientY)}
                        onDragMove={moveDrag}
                        onDragEnd={endDrag}
                      />
                    )}
                  </For>
                </div>
              </Show>

              <RenamePlaylistDialog
                open={renaming()}
                onOpenChange={setRenaming}
                playlistId={current().id}
                initialName={current().name}
                onRenamed={() => {
                  void refetchPlaylist()
                }}
              />

              <ConfirmDialog
                open={deleting()}
                onOpenChange={setDeleting}
                title={`Удалить «${current().name}»?`}
                description="Плейлист исчезнет, сами треки останутся в библиотеке."
                confirmLabel="Удалить"
                danger
                onConfirm={async () => {
                  const ok = await deletePlaylist(current().id)
                  if (ok) navigate('/playlists', { replace: true })
                  else await refetchTracks()
                }}
              />
            </>
          )}
        </Show>
      </Show>
    </Screen>
  )
}

/** Шапка: обложка-мозаика, название, объём и кнопки воспроизведения. */
function Header(props: {
  playlist: Playlist
  tracks: Resource<Track[]>
  onPlay: () => void
  onShuffle: () => void
}) {
  const count = (): number => props.tracks()?.length ?? props.playlist.trackCount

  const duration = (): number => {
    const list = props.tracks()
    if (list === undefined) return props.playlist.durationMs
    return list.reduce((total, track) => total + track.durationMs, 0)
  }

  const subtitle = (): string => {
    const parts: string[] = [formatPlural(count(), TRACK_FORMS)]
    if (duration() > 0) parts.push(formatDurationHuman(duration()))
    return parts.join(' · ')
  }

  return (
    <section class="flex flex-col items-center gap-4 px-4 pt-2 pb-5">
      <div class="w-40">
        <PlaylistCover
          playlistId={props.playlist.id}
          coverKey={props.playlist.coverKey}
          trackCount={props.playlist.trackCount}
          name={props.playlist.name}
          fill
          class="rounded-2xl"
        />
      </div>

      <div class="flex w-full flex-col items-center gap-1">
        <h2 class="w-full truncate text-center text-xl font-semibold text-foreground">
          {props.playlist.name}
        </h2>
        <p class="text-xs text-muted">{subtitle()}</p>
      </div>

      <div class="flex w-full items-center gap-2">
        <Button
          variant="primary"
          fullWidth
          disabled={count() === 0}
          onClick={() => props.onPlay()}
        >
          <Play size={18} aria-hidden="true" />
          Слушать
        </Button>
        <Button
          variant="secondary"
          fullWidth
          disabled={count() === 0}
          onClick={() => props.onShuffle()}
        >
          <Shuffle size={18} aria-hidden="true" />
          Перемешать
        </Button>
      </div>
    </section>
  )
}

type ReorderableRowProps = {
  track: Track
  index: number
  active: boolean
  dragging: boolean
  /** Сдвиг соседних строк, пока тащат другую. */
  shift: number
  /** Сдвиг самой перетаскиваемой строки. */
  dragOffset: number
  onPlay: () => void
  onRemove: () => void
  onDragStart: (clientY: number) => void
  onDragMove: (clientY: number) => void
  onDragEnd: () => void
}

/**
 * Строка плейлиста: вертикальное перетаскивание за ручку и свайп влево на удаление.
 * `touch-action` разводит жесты — `none` на ручке, `pan-y` на строке, поэтому
 * скролл списка остаётся системным и не требует `preventDefault`.
 */
function ReorderableRow(props: ReorderableRowProps) {
  const [swipeX, setSwipeX] = createSignal(0)

  let startX = 0
  let startY = 0
  let axis: 'none' | 'swipe' | 'scroll' = 'none'
  // Android досылает `click` после свайпа — тап по строке после жеста игнорируем.
  let swiped = false

  const onTouchStart = (event: TouchEvent): void => {
    const touch = event.touches[0]
    if (touch === undefined || event.touches.length > 1) return
    startX = touch.clientX
    startY = touch.clientY
    axis = 'none'
    swiped = false
  }

  const onTouchMove = (event: TouchEvent): void => {
    const touch = event.touches[0]
    if (touch === undefined) return

    const dx = touch.clientX - startX
    const dy = touch.clientY - startY

    if (axis === 'none') {
      if (Math.abs(dx) < SWIPE_SLOP_PX && Math.abs(dy) < SWIPE_SLOP_PX) return
      axis = Math.abs(dx) > Math.abs(dy) ? 'swipe' : 'scroll'
    }

    if (axis === 'swipe') setSwipeX(Math.min(0, dx))
  }

  const onTouchEnd = (): void => {
    const offset = swipeX()
    swiped = axis === 'swipe' && Math.abs(offset) > SWIPE_SLOP_PX
    axis = 'none'
    setSwipeX(0)

    if (offset <= -SWIPE_REMOVE_PX) props.onRemove()
  }

  const play = (): void => {
    if (swiped) {
      swiped = false
      return
    }
    props.onPlay()
  }

  const handleTouchStart = (event: TouchEvent): void => {
    const touch = event.touches[0]
    if (touch === undefined) return
    props.onDragStart(touch.clientY)
  }

  const handleTouchMove = (event: TouchEvent): void => {
    const touch = event.touches[0]
    if (touch === undefined) return
    props.onDragMove(touch.clientY)
  }

  const outerStyle = (): JSX.CSSProperties => ({
    height: `${String(TRACK_ROW_HEIGHT)}px`,
    transform: `translateY(${String(props.dragging ? props.dragOffset : props.shift)}px)`,
    transition: props.dragging ? 'none' : 'transform 160ms var(--ease-smooth)',
    'z-index': props.dragging ? 10 : 0,
  })

  return (
    <div class="relative w-full" style={outerStyle()}>
      <div class="absolute inset-0 flex items-center justify-end bg-danger-soft pr-5 text-danger-soft-foreground">
        <Trash2 size={20} aria-hidden="true" />
      </div>

      <div
        class="relative flex h-full w-full items-center bg-background"
        classList={{ 'shadow-[var(--shadow-overlay)]': props.dragging }}
        style={{
          transform: `translateX(${String(swipeX())}px)`,
          transition: swipeX() === 0 ? 'transform 160ms var(--ease-smooth)' : 'none',
          'touch-action': 'pan-y',
        }}
        onTouchStart={onTouchStart}
        onTouchMove={onTouchMove}
        onTouchEnd={onTouchEnd}
        onTouchCancel={onTouchEnd}
      >
        <TrackRow
          class="min-w-0 flex-1 pr-0"
          track={props.track}
          index={props.index}
          active={props.active}
          onPlay={play}
        />

        <button
          type="button"
          class="flex h-11 w-11 shrink-0 items-center justify-center text-muted no-highlight"
          style={{ 'touch-action': 'none' }}
          aria-label={`Переместить: ${props.track.title}`}
          onTouchStart={handleTouchStart}
          onTouchMove={handleTouchMove}
          onTouchEnd={() => props.onDragEnd()}
          onTouchCancel={() => props.onDragEnd()}
        >
          <GripVertical size={18} aria-hidden="true" />
        </button>
      </div>
    </div>
  )
}
