import { EllipsisVertical } from 'lucide-solid'
import { Show } from 'solid-js'

import type { Track } from '@/shared/ipc'
import { cn, formatDuration } from '@/shared/lib'
import { CoverArt } from './CoverArt'
import { IconButton } from './IconButton'

/** Высота строки фиксирована — на неё завязана виртуализация списков. */
export const TRACK_ROW_HEIGHT = 64

export type TrackRowProps = {
  track: Track
  index?: number
  /** Трек играет прямо сейчас. */
  active?: boolean
  showArtwork?: boolean
  onPlay?: (track: Track, index: number | undefined) => void
  onMenu?: (track: Track, index: number | undefined) => void
  class?: string
}

/** Строка трека: обложка, название, «исполнитель · альбом», длительность, меню. */
export function TrackRow(props: TrackRowProps) {
  const isActive = (): boolean => props.active === true

  /**
   * Заглушку-градиент вяжем к альбому, а не к треку: у альбома без обложки все
   * его треки должны получить одно и то же пятно, иначе список рябит.
   */
  const coverSeed = (): string =>
    `${props.track.albumTitle ?? props.track.title}·${props.track.artistName ?? ''}`

  const subtitle = (): string => {
    const parts: string[] = []
    const artist = props.track.artistName
    const album = props.track.albumTitle
    if (artist !== null && artist !== '') parts.push(artist)
    if (album !== null && album !== '') parts.push(album)
    return parts.length === 0 ? 'Неизвестный исполнитель' : parts.join(' · ')
  }

  return (
    <div
      class={cn(
        'flex h-16 w-full items-center gap-1 pr-1 pl-4',
        'transition-colors duration-150 active:bg-surface-secondary',
        isActive() && 'bg-surface-secondary',
        props.class,
      )}
      data-active={isActive() ? 'true' : undefined}
    >
      <button
        type="button"
        class="flex h-full min-w-0 flex-1 items-center gap-3 text-start no-highlight"
        onClick={() => props.onPlay?.(props.track, props.index)}
      >
        <Show when={props.showArtwork !== false}>
          <CoverArt
            coverKey={props.track.coverKey}
            seed={coverSeed()}
            size="sm"
            rounded="md"
          />
        </Show>

        <span class="flex min-w-0 flex-1 flex-col gap-0.5">
          <span
            class={cn(
              'truncate text-sm font-medium',
              isActive() ? 'text-accent' : 'text-foreground',
            )}
          >
            {props.track.title}
          </span>
          <span class="truncate text-xs text-muted">{subtitle()}</span>
        </span>

        <span class="shrink-0 text-xs tabular-nums text-muted">
          {formatDuration(props.track.durationMs)}
        </span>
      </button>

      <Show when={props.onMenu}>
        {(onMenu) => (
          <IconButton
            label={`Действия: ${props.track.title}`}
            onClick={() => onMenu()(props.track, props.index)}
          >
            <EllipsisVertical aria-hidden="true" />
          </IconButton>
        )}
      </Show>
    </div>
  )
}
