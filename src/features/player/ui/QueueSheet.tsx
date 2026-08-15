import { ArrowDown, ArrowUp, ListMusic, Trash2 } from 'lucide-solid'
import { Show } from 'solid-js'

import {
  playerClearQueue,
  playerMoveQueueItem,
  playerRemoveQueueItem,
  playerSkipTo,
  type Track,
} from '@/shared/ipc'
import { cn, formatDuration, formatPlural } from '@/shared/lib'
import { Button } from '@/shared/ui/Button'
import { CoverArt } from '@/shared/ui/CoverArt'
import { EmptyState } from '@/shared/ui/EmptyState'
import { IconButton } from '@/shared/ui/IconButton'
import { Sheet } from '@/shared/ui/Sheet'
import { toast } from '@/shared/ui/Toasts'
import { VirtualList } from '@/shared/ui/VirtualList'
import { usePlayer } from '../model/player-store'

const TRACK_FORMS: readonly [string, string, string] = ['трек', 'трека', 'треков']

/** Строка очереди чуть выше обычной: в ней ещё две кнопки перестановки. */
const QUEUE_ROW_HEIGHT = 64

export type QueueSheetProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
}

/**
 * Очередь воспроизведения. Список — то, что отдал нативный плеер;
 * любое изменение уходит командой, а обратно приезжает событием
 * `player:queue-changed`, которое перечитывает стор.
 */
export function QueueSheet(props: QueueSheetProps) {
  const player = usePlayer()

  const items = (): readonly Track[] => player.queue

  const activeIndex = (): number | null => player.state.queueIndex

  const run = (message: string, action: () => Promise<void>): void => {
    action().catch((error: unknown) => {
      console.error(`[player] ${message}`, error)
      toast({ title: message, variant: 'danger' })
    })
  }

  /**
   * Выбор трека — это «покажи мне его»: очередь закрывается всегда, а не только
   * когда ткнули в уже играющий. Иначе лист остаётся висеть поверх плеера,
   * хотя выбор уже сделан, и обложку выбранного трека не видно.
   */
  const skipTo = (index: number): void => {
    props.onOpenChange(false)
    if (index === activeIndex()) return
    run('Не удалось переключить трек', () => playerSkipTo(index))
  }

  const move = (from: number, to: number): void => {
    if (to < 0 || to >= items().length) return
    run('Не удалось переставить трек', () => playerMoveQueueItem(from, to))
  }

  const remove = (index: number): void => {
    run('Не удалось убрать трек из очереди', () => playerRemoveQueueItem(index))
  }

  const clear = (): void => {
    run('Не удалось очистить очередь', async () => {
      await playerClearQueue()
      props.onOpenChange(false)
    })
  }

  return (
    <Sheet
      open={props.open}
      onOpenChange={props.onOpenChange}
      title="Очередь"
      description={formatPlural(items().length, TRACK_FORMS)}
      footer={
        <Show when={items().length > 0}>
          <Button variant="ghost" fullWidth onClick={clear}>
            Очистить очередь
          </Button>
        </Show>
      }
    >
      <Show
        when={items().length > 0}
        fallback={
          <EmptyState
            icon={<ListMusic aria-hidden="true" />}
            title="Очередь пуста"
            description="Запустите трек или добавьте что-нибудь из библиотеки."
          />
        }
      >
        <div class="-mx-2 h-[52dvh]">
          <VirtualList items={items()} estimateSize={QUEUE_ROW_HEIGHT}>
            {(track, index) => (
              <QueueRow
                track={track}
                index={index}
                active={index === activeIndex()}
                first={index === 0}
                last={index === items().length - 1}
                onPlay={skipTo}
                onMove={move}
                onRemove={remove}
              />
            )}
          </VirtualList>
        </div>
      </Show>
    </Sheet>
  )
}

type QueueRowProps = {
  track: Track
  index: number
  active: boolean
  first: boolean
  last: boolean
  onPlay: (index: number) => void
  onMove: (from: number, to: number) => void
  onRemove: (index: number) => void
}

function QueueRow(props: QueueRowProps) {
  const subtitle = (): string => props.track.artistName ?? 'Неизвестный исполнитель'

  return (
    <div
      class={cn(
        'flex h-16 w-full items-center gap-1 rounded-xl pr-1 pl-2',
        props.active && 'bg-surface-secondary',
      )}
      data-active={props.active ? 'true' : undefined}
    >
      <button
        type="button"
        class="flex h-full min-w-0 flex-1 items-center gap-3 text-start no-highlight"
        onClick={() => props.onPlay(props.index)}
      >
        <CoverArt
          coverKey={props.track.coverKey}
          seed={`${props.track.albumTitle ?? props.track.title}·${props.track.artistName ?? ''}`}
          size="sm"
          rounded="md"
        />
        <span class="flex min-w-0 flex-1 flex-col gap-0.5">
          <span
            class={cn(
              'truncate text-sm font-medium',
              props.active ? 'text-accent' : 'text-foreground',
            )}
          >
            {props.track.title}
          </span>
          <span class="truncate text-xs text-muted">
            {subtitle()} · {formatDuration(props.track.durationMs)}
          </span>
        </span>
      </button>

      <IconButton
        label={`Выше: ${props.track.title}`}
        size="sm"
        disabled={props.first}
        onClick={() => props.onMove(props.index, props.index - 1)}
      >
        <ArrowUp size={18} aria-hidden="true" />
      </IconButton>
      <IconButton
        label={`Ниже: ${props.track.title}`}
        size="sm"
        disabled={props.last}
        onClick={() => props.onMove(props.index, props.index + 1)}
      >
        <ArrowDown size={18} aria-hidden="true" />
      </IconButton>
      <IconButton
        label={`Убрать из очереди: ${props.track.title}`}
        size="sm"
        onClick={() => props.onRemove(props.index)}
      >
        <Trash2 size={18} aria-hidden="true" />
      </IconButton>
    </div>
  )
}
