import { Pause, Play, Repeat, Repeat1, Shuffle, SkipBack, SkipForward } from 'lucide-solid'
import { Show } from 'solid-js'

import type { RepeatMode } from '@/shared/ipc'
import { cn } from '@/shared/lib'
import { Button } from '@/shared/ui/Button'
import { IconButton } from '@/shared/ui/IconButton'
import { usePlayer } from '../model/player-store'

export type PlayerControlsProps = {
  /** `mini` — только перемотка треков, `full` — с перемешиванием и повтором. */
  variant?: 'mini' | 'full'
  class?: string
}

const NEXT_REPEAT: Record<RepeatMode, RepeatMode> = {
  OFF: 'ALL',
  ALL: 'ONE',
  ONE: 'OFF',
}

const REPEAT_LABEL: Record<RepeatMode, string> = {
  OFF: 'Повтор выключен',
  ALL: 'Повтор очереди',
  ONE: 'Повтор трека',
}

/** Ряд управления воспроизведением. Состояние берёт из стора плеера. */
export function PlayerControls(props: PlayerControlsProps) {
  const player = usePlayer()

  const isFull = (): boolean => props.variant === 'full'
  const isPlaying = (): boolean =>
    player.state.status === 'PLAYING' || player.state.status === 'BUFFERING'

  return (
    <div
      class={cn(
        'flex items-center',
        isFull() ? 'justify-between gap-2' : 'gap-0.5',
        props.class,
      )}
      data-no-swipe="true"
    >
      <Show when={isFull()}>
        <IconButton
          label={player.state.shuffle ? 'Выключить перемешивание' : 'Перемешать очередь'}
          class={player.state.shuffle ? 'text-accent' : undefined}
          onClick={() => player.setShuffle(!player.state.shuffle)}
        >
          <Shuffle size={20} aria-hidden="true" />
        </IconButton>
      </Show>

      <IconButton label="Предыдущий трек" onClick={() => player.prev()}>
        <SkipBack size={isFull() ? 26 : 22} fill="currentColor" aria-hidden="true" />
      </IconButton>

      <Button
        variant={isFull() ? 'primary' : 'ghost'}
        iconOnly
        aria-label={isPlaying() ? 'Пауза' : 'Воспроизвести'}
        class={cn('rounded-full', isFull() ? 'size-16' : 'min-h-11 min-w-11')}
        onClick={() => player.toggle()}
      >
        <Show
          when={isPlaying()}
          fallback={<Play size={isFull() ? 28 : 24} fill="currentColor" aria-hidden="true" />}
        >
          <Pause size={isFull() ? 28 : 24} fill="currentColor" aria-hidden="true" />
        </Show>
      </Button>

      <IconButton label="Следующий трек" onClick={() => player.next()}>
        <SkipForward size={isFull() ? 26 : 22} fill="currentColor" aria-hidden="true" />
      </IconButton>

      <Show when={isFull()}>
        <IconButton
          label={REPEAT_LABEL[player.state.repeat]}
          class={player.state.repeat === 'OFF' ? undefined : 'text-accent'}
          onClick={() => player.setRepeat(NEXT_REPEAT[player.state.repeat])}
        >
          <Show
            when={player.state.repeat === 'ONE'}
            fallback={<Repeat size={20} aria-hidden="true" />}
          >
            <Repeat1 size={20} aria-hidden="true" />
          </Show>
        </IconButton>
      </Show>
    </div>
  )
}
