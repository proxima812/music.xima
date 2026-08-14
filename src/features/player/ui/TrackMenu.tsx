import { useNavigate } from '@solidjs/router'
import {
  Disc3,
  EllipsisVertical,
  EyeOff,
  Heart,
  Info,
  ListEnd,
  ListMusic,
  ListStart,
  MicVocal,
  Plus,
  Trash2,
} from 'lucide-solid'
import {
  createResource,
  createSignal,
  For,
  Match,
  Show,
  Switch,
  type JSX,
} from 'solid-js'

import {
  playlistAddTracks,
  playlistCreate,
  playlistList,
  toIpcError,
  trackDeleteFile,
  trackHide,
  trackRestore,
  type Playlist,
  type Track,
} from '@/shared/ipc'
import { formatBytes, formatCount, formatDate, formatDuration } from '@/shared/lib'
import { Button } from '@/shared/ui/Button'
import { EmptyState } from '@/shared/ui/EmptyState'
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog'
import { IconButton } from '@/shared/ui/IconButton'
import { Sheet } from '@/shared/ui/Sheet'
import { Spinner } from '@/shared/ui/Spinner'
import { toast } from '@/shared/ui/Toasts'
import { isFavorite, toggleFavorite } from '../model/favorites'
import { usePlayer } from '../model/player-store'

export type TrackMenuProps = {
  track: Track | null
  /** Контролируемый режим: если передан, собственная кнопка-триггер не рисуется. */
  open?: boolean
  onOpenChange?: (open: boolean) => void
  /** Дёргается перед переходом на другой экран — чтобы закрыть оверлей плеера. */
  onNavigate?: () => void
  triggerLabel?: string
  triggerClass?: string
}

type Panel = 'actions' | 'playlists' | 'details'

const MAX_PLAYLIST_NAME = 80

/** Меню трека: очередь, избранное, плейлисты, переходы и сведения о файле. */
export function TrackMenu(props: TrackMenuProps) {
  const player = usePlayer()

  // Компонент могут отрисовать вне роутера (оверлей плеера) — переходы тогда прячем.
  let navigate: ReturnType<typeof useNavigate> | null = null
  try {
    navigate = useNavigate()
  } catch {
    navigate = null
  }

  const [selfOpen, setSelfOpen] = createSignal(false)
  const [panel, setPanel] = createSignal<Panel>('actions')
  const [newName, setNewName] = createSignal('')
  const [trackToDelete, setTrackToDelete] = createSignal<Track | null>(null)

  const isControlled = (): boolean => props.open !== undefined
  const isOpen = (): boolean => (props.open ?? selfOpen()) && props.track !== null

  const setOpen = (open: boolean): void => {
    if (!open) {
      setPanel('actions')
      setNewName('')
    }
    setSelfOpen(open)
    props.onOpenChange?.(open)
  }

  const [playlists] = createResource(
    () => (panel() === 'playlists' && isOpen() ? 'playlists' : undefined),
    () => playlistList(),
  )

  const withTrack = (action: (track: Track) => void): (() => void) => {
    return () => {
      const track = props.track
      if (track === null) return
      action(track)
    }
  }

  const go = (path: string): void => {
    const to = navigate
    if (to === null) return
    setOpen(false)
    props.onNavigate?.()
    to(path)
  }

  const addToPlaylist = (playlist: Playlist, track: Track): void => {
    playlistAddTracks(playlist.id, [track.id])
      .then(() => {
        toast({ title: `Добавлено в «${playlist.name}»` })
        setOpen(false)
      })
      .catch((error: unknown) => {
        console.error('[player] не удалось добавить трек в плейлист', error)
        toast({ title: 'Не удалось добавить в плейлист', variant: 'danger' })
      })
  }

  const createAndAdd = (track: Track): void => {
    const name = newName().trim()
    if (name === '') return

    playlistCreate(name)
      .then(async (playlist) => {
        await playlistAddTracks(playlist.id, [track.id])
        toast({ title: `Добавлено в «${playlist.name}»` })
        setNewName('')
        setOpen(false)
      })
      .catch((error: unknown) => {
        console.error('[player] не удалось создать плейлист', error)
        toast({ title: 'Не удалось создать плейлист', variant: 'danger' })
      })
  }

  const hideTrack = (track: Track): void => {
    void trackHide(track.id)
      .then(() => {
        setOpen(false)
        toast({
          title: 'Песня скрыта из music.xima',
          action: {
            label: 'Вернуть',
            ariaLabel: `Вернуть песню ${track.title}`,
            onClick: async () => {
              try {
                await trackRestore(track.id)
                toast({ title: 'Песня возвращена', variant: 'success' })
              } catch (error: unknown) {
                toast({
                  title: 'Не удалось вернуть песню',
                  description: toIpcError(error).message,
                  variant: 'danger',
                })
                throw error
              }
            },
          },
          duration: 8000,
        })
      })
      .catch((error: unknown) => {
        console.error('[player] не удалось скрыть трек', error)
        toast({
          title: 'Не удалось скрыть песню',
          description: toIpcError(error).message,
          variant: 'danger',
        })
      })
  }

  const deleteTrackFile = (track: Track): Promise<void> =>
    trackDeleteFile(track.id)
      .then((result) => {
        if (result === 'cancelled') return

        setOpen(false)
        props.onNavigate?.()
        toast({ title: 'Файл удалён с устройства', variant: 'success' })
      })
      .catch((error: unknown) => {
        const ipcError = toIpcError(error)
        const unsupported = ipcError.code === 'UNSUPPORTED_DELETE'
        toast(
          unsupported
            ? {
                title: 'Этот файл нельзя удалить через Android. Его можно скрыть из music.xima.',
                variant: 'danger',
              }
            : {
                title: 'Не удалось удалить файл',
                description: ipcError.message,
                variant: 'danger',
              },
        )
        throw error
      })

  const deleteDescription = (): string => {
    const track = trackToDelete()
    const title = track?.title ?? ''
    return `«${title}» будет удалена с устройства без возможности восстановления.`
  }

  return (
    <>
      <Show when={!isControlled()}>
        <IconButton
          label={props.triggerLabel ?? 'Действия с треком'}
          class={props.triggerClass}
          disabled={props.track === null}
          onClick={() => setOpen(true)}
        >
          <EllipsisVertical aria-hidden="true" />
        </IconButton>
      </Show>

      <Sheet
        open={isOpen()}
        onOpenChange={setOpen}
        title={props.track?.title ?? ''}
        description={props.track?.artistName ?? 'Неизвестный исполнитель'}
      >
        <Show when={props.track}>
          {(track) => (
            <Switch>
              <Match when={panel() === 'actions'}>
                <div class="menu -mx-2 gap-0.5 p-0">
                  <ActionRow
                    icon={<ListStart size={18} aria-hidden="true" />}
                    label="Играть дальше"
                    onClick={withTrack((value) => {
                      player.addNext([value.id])
                      toast({ title: 'Играет следующим' })
                      setOpen(false)
                    })}
                  />
                  <ActionRow
                    icon={<ListEnd size={18} aria-hidden="true" />}
                    label="В конец очереди"
                    onClick={withTrack((value) => {
                      player.addToQueue([value.id])
                      toast({ title: 'Добавлено в очередь' })
                      setOpen(false)
                    })}
                  />
                  <ActionRow
                    icon={
                      <Heart
                        size={18}
                        fill={isFavorite(track()) ? 'currentColor' : 'none'}
                        aria-hidden="true"
                      />
                    }
                    label={isFavorite(track()) ? 'Убрать из избранного' : 'В избранное'}
                    onClick={withTrack((value) => {
                      toggleFavorite(value)
                      setOpen(false)
                    })}
                  />
                  <ActionRow
                    icon={<ListMusic size={18} aria-hidden="true" />}
                    label="Добавить в плейлист"
                    onClick={() => setPanel('playlists')}
                  />

                  <Show when={navigate !== null && track().albumId !== null}>
                    <ActionRow
                      icon={<Disc3 size={18} aria-hidden="true" />}
                      label="Перейти к альбому"
                      onClick={() => go(`/library/album/${String(track().albumId)}`)}
                    />
                  </Show>

                  <Show when={navigate !== null && track().artistId !== null}>
                    <ActionRow
                      icon={<MicVocal size={18} aria-hidden="true" />}
                      label="Перейти к исполнителю"
                      onClick={() => go(`/library/artist/${String(track().artistId)}`)}
                    />
                  </Show>

                  <ActionRow
                    icon={<Info size={18} aria-hidden="true" />}
                    label="Сведения о файле"
                    onClick={() => setPanel('details')}
                  />

                  <ActionRow
                    icon={<EyeOff size={18} aria-hidden="true" />}
                    label="Скрыть из music.xima"
                    onClick={withTrack(hideTrack)}
                  />

                  <div class="my-2 h-px bg-separator" role="separator" />

                  <ActionRow
                    icon={<Trash2 size={18} aria-hidden="true" />}
                    label="Удалить файл с устройства"
                    danger
                    onClick={withTrack((value) => {
                      setTrackToDelete(value)
                      setOpen(false)
                    })}
                  />
                </div>
              </Match>

              <Match when={panel() === 'playlists'}>
                <div class="flex flex-col gap-3">
                  <div class="flex items-center gap-2">
                    <input
                      class="input input--secondary w-full"
                      placeholder="Новый плейлист"
                      maxLength={MAX_PLAYLIST_NAME}
                      value={newName()}
                      onInput={(event) => setNewName(event.currentTarget.value)}
                    />
                    <Button
                      variant="primary"
                      iconOnly
                      aria-label="Создать плейлист и добавить трек"
                      class="min-h-11 min-w-11 shrink-0"
                      disabled={newName().trim() === ''}
                      onClick={() => createAndAdd(track())}
                    >
                      <Plus aria-hidden="true" />
                    </Button>
                  </div>

                  <Show
                    when={playlists()}
                    fallback={
                      <div class="flex justify-center py-6">
                        <Spinner />
                      </div>
                    }
                  >
                    {(items) => (
                      <Show
                        when={items().length > 0}
                        fallback={
                          <EmptyState
                            icon={<ListMusic aria-hidden="true" />}
                            title="Плейлистов пока нет"
                            description="Создайте первый — трек попадёт сразу в него."
                          />
                        }
                      >
                        <div class="menu -mx-2 gap-0.5 p-0">
                          <For each={items()}>
                            {(playlist) => (
                              <ActionRow
                                icon={<ListMusic size={18} aria-hidden="true" />}
                                label={playlist.name}
                                hint={formatCount(playlist.trackCount)}
                                onClick={() => addToPlaylist(playlist, track())}
                              />
                            )}
                          </For>
                        </div>
                      </Show>
                    )}
                  </Show>

                  <Button variant="ghost" onClick={() => setPanel('actions')}>
                    Назад
                  </Button>
                </div>
              </Match>

              <Match when={panel() === 'details'}>
                <div class="flex flex-col gap-3">
                  <dl class="flex flex-col divide-y divide-separator">
                    <DetailRow label="Длительность" value={formatDuration(track().durationMs)} />
                    <DetailRow label="Исполнитель" value={track().artistName} />
                    <DetailRow label="Альбом" value={track().albumTitle} />
                    <DetailRow label="Жанр" value={track().genre} />
                    <DetailRow
                      label="Год"
                      value={track().year === null ? null : String(track().year)}
                    />
                    <DetailRow label="Формат" value={track().format} />
                    <DetailRow label="Битрейт" value={formatBitrate(track().bitrate)} />
                    <DetailRow label="Частота" value={formatSampleRate(track().sampleRate)} />
                    <DetailRow label="Размер" value={formatBytes(track().size)} />
                    <DetailRow label="Папка" value={track().folder} />
                    <DetailRow label="Добавлен" value={formatDate(track().dateAdded)} />
                    <DetailRow
                      label="Прослушиваний"
                      value={formatCount(track().playCount)}
                    />
                    <DetailRow label="Файл" value={track().uri} />
                  </dl>

                  <Button variant="ghost" onClick={() => setPanel('actions')}>
                    Назад
                  </Button>
                </div>
              </Match>
            </Switch>
          )}
        </Show>
      </Sheet>

      <ConfirmDialog
        open={trackToDelete() !== null}
        onOpenChange={(open) => {
          if (!open) setTrackToDelete(null)
        }}
        title="Удалить файл с устройства?"
        description={deleteDescription()}
        confirmLabel="Удалить файл"
        danger
        onConfirm={() => {
          const track = trackToDelete()
          return track === null ? undefined : deleteTrackFile(track)
        }}
      />
    </>
  )
}

function ActionRow(props: {
  icon: JSX.Element
  label: string
  hint?: string
  danger?: boolean
  onClick: () => void
}) {
  return (
    <button type="button" class="menu-item min-h-11" onClick={() => props.onClick()}>
      <span
        class={`flex size-5 shrink-0 items-center justify-center ${
          props.danger === true ? 'text-danger' : 'text-muted'
        }`}
      >
        {props.icon}
      </span>
      <span
        data-slot="label"
        class={`flex-1 truncate text-start text-sm ${
          props.danger === true ? 'text-danger' : 'text-foreground'
        }`}
      >
        {props.label}
      </span>
      <Show when={props.hint}>
        {(hint) => <span class="shrink-0 text-xs text-muted">{hint()}</span>}
      </Show>
    </button>
  )
}

function DetailRow(props: { label: string; value: string | null }) {
  return (
    <Show when={props.value !== null && props.value !== ''}>
      <div class="flex items-start justify-between gap-4 py-2">
        <dt class="shrink-0 text-xs text-muted">{props.label}</dt>
        <dd class="min-w-0 text-end text-xs break-all text-foreground">{props.value}</dd>
      </div>
    </Show>
  )
}

function formatBitrate(bitrate: number | null): string | null {
  if (bitrate === null || bitrate <= 0) return null
  const kbps = bitrate >= 1000 ? Math.round(bitrate / 1000) : bitrate
  return `${String(kbps)} кбит/с`
}

function formatSampleRate(sampleRate: number | null): string | null {
  if (sampleRate === null || sampleRate <= 0) return null
  const khz = sampleRate >= 1000 ? sampleRate / 1000 : sampleRate
  return `${khz.toFixed(1).replace('.0', '')} кГц`
}
