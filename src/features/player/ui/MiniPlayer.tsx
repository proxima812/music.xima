import { Show } from 'solid-js'

import { CoverArt } from '@/shared/ui/CoverArt'
import { createSwipe } from '../model/gestures'
import { usePlayer } from '../model/player-store'
import { PlayerControls } from './PlayerControls'

/**
 * Мини-плеер: полоска прогресса сверху, обложка 48px, название с исполнителем
 * и перемотка треков. Тап по телу открывает полноэкранный плеер,
 * свайп влево/вправо переключает трек, свайп вверх тоже открывает плеер.
 *
 * Позиционирование — на каркасе приложения: здесь только сама панель.
 */
export function MiniPlayer() {
  const player = usePlayer()

  const progress = (): number => {
    const duration = player.state.durationMs
    if (duration <= 0) return 0
    return Math.min(100, Math.max(0, (player.state.positionMs / duration) * 100))
  }

  const swipe = createSwipe({
    directions: ['left', 'right', 'up'],
    onSwipe: (direction) => {
      if (direction === 'left') player.next()
      else if (direction === 'right') player.prev()
      else player.openFull()
    },
  })

  return (
    <Show when={player.current}>
      {(track) => (
        <div
          class="depth-floating animate-in slide-in-from-bottom-4 fade-in-0 relative h-mini-player w-full shrink-0 touch-none overflow-hidden border-t border-border duration-200 ease-out-fluid"
          onTouchStart={swipe.onTouchStart}
          onTouchMove={swipe.onTouchMove}
          onTouchEnd={swipe.onTouchEnd}
          onTouchCancel={swipe.onTouchCancel}
        >
          <div class="absolute inset-x-0 top-0 h-0.5 bg-separator">
            <div
              class="h-full bg-accent transition-[width] duration-200 ease-linear"
              style={{ width: `${progress()}%` }}
            />
          </div>

          <div class="mx-auto flex h-full w-full max-w-3xl items-center gap-2 pr-1 pl-2">
            <button
              type="button"
              class="flex h-full min-w-0 flex-1 items-center gap-3 text-start no-highlight"
              aria-label={`Открыть плеер: ${track().title}`}
              onClick={() => player.openFull()}
            >
              <CoverArt
                coverKey={track().coverKey}
                seed={`${track().albumTitle ?? track().title}·${track().artistName ?? ''}`}
                size={48}
                rounded="md"
              />

              <span class="flex min-w-0 flex-1 flex-col gap-0.5">
                <span class="truncate text-sm font-medium text-foreground">{track().title}</span>
                <span class="truncate text-xs text-muted">
                  {track().artistName ?? 'Неизвестный исполнитель'}
                </span>
              </span>
            </button>

            <PlayerControls variant="mini" />
          </div>
        </div>
      )}
    </Show>
  )
}
