import { Dialog } from '@kobalte/core/dialog'
import { ChevronDown, Heart, ListMusic, Music } from 'lucide-solid'
import { createSignal, Show } from 'solid-js'

import { CoverArt } from '@/shared/ui/CoverArt'
import { EmptyState } from '@/shared/ui/EmptyState'
import { IconButton } from '@/shared/ui/IconButton'
import { isFavorite, toggleFavorite } from '../model/favorites'
import { createSwipe } from '../model/gestures'
import { usePlayer } from '../model/player-store'
import { PlayerControls } from './PlayerControls'
import { QueueSheet } from './QueueSheet'
import { SeekBar } from './SeekBar'
import { TrackMenu } from './TrackMenu'

/** Полноэкранный плеер: обложка, метаданные, перемотка, управление и очередь. */
export function FullPlayer() {
  const player = usePlayer()

  const [queueOpen, setQueueOpen] = createSignal(false)
  const [dragY, setDragY] = createSignal(0)

  const swipe = createSwipe({
    directions: ['down'],
    threshold: 90,
    onMove: ({ dx, dy }) => {
      setDragY(dy > 0 && dy > Math.abs(dx) ? dy : 0)
    },
    onSwipe: () => {
      player.closeFull()
    },
    onEnd: () => {
      setDragY(0)
    },
  })

  const subtitle = (): string => player.current?.artistName ?? 'Неизвестный исполнитель'

  return (
    <Dialog
      open={player.isFullOpen}
      onOpenChange={(open) => {
        if (!open) player.closeFull()
      }}
      modal
      preventScroll
    >
      <Dialog.Portal>
        <Dialog.Overlay class="modal__backdrop modal__backdrop--opaque h-dvh animate-in fade-in-0 duration-200" />

        <div class="fixed inset-0 z-50 modal__container h-dvh p-0 sm:w-full sm:p-0">
          <Dialog.Content
            class="modal__dialog modal__dialog--full safe-top safe-bottom animate-in slide-in-from-bottom duration-300 ease-out-fluid gap-4 px-5 pt-2 pb-4"
            style={{ transform: `translateY(${String(dragY())}px)` }}
            onTouchStart={swipe.onTouchStart}
            onTouchMove={swipe.onTouchMove}
            onTouchEnd={swipe.onTouchEnd}
            onTouchCancel={swipe.onTouchCancel}
          >
            <div class="flex items-center justify-between gap-2">
              <IconButton label="Свернуть плеер" onClick={() => player.closeFull()}>
                <ChevronDown aria-hidden="true" />
              </IconButton>
              <span class="truncate text-xs tracking-wide text-muted uppercase">Сейчас играет</span>
              <span class="size-11 shrink-0" aria-hidden="true" />
            </div>

            <Show
              when={player.current}
              fallback={
                <div class="flex flex-1 items-center justify-center">
                  <EmptyState
                    icon={<Music aria-hidden="true" />}
                    title="Ничего не играет"
                    description="Выберите трек в библиотеке — он появится здесь."
                  />
                </div>
              }
            >
              {(track) => (
                <>
                  {/*
                    Обложка обязана остаться квадратом на любом экране, поэтому
                    сторона считается как меньшая из размеров области:
                    `min(100cqw, 100cqh)`. Через `h-full` + `max-w-full` это не
                    выражается — при клампе ширины высота остаётся прежней и
                    квадрат превращается в прямоугольник.
                  */}
                  <div class="flex min-h-0 flex-1 items-center justify-center [container-type:size]">
                    <div class="w-[min(100cqw,100cqh)]">
                      <CoverArt
                        coverKey={track().coverKey}
                        seed={`${track().albumTitle ?? track().title}·${track().artistName ?? ''}`}
                        size="full"
                        rounded="lg"
                        alt={`Обложка: ${track().title}`}
                      />
                    </div>
                  </div>

                  <div class="flex shrink-0 flex-col gap-1">
                    <Dialog.Title class="truncate text-lg font-semibold text-foreground">
                      {track().title}
                    </Dialog.Title>
                    <Dialog.Description class="truncate text-sm text-muted">
                      {subtitle()}
                    </Dialog.Description>
                    <Show when={track().albumTitle}>
                      {(album) => <span class="truncate text-xs text-muted">{album()}</span>}
                    </Show>
                  </div>

                  <SeekBar
                    positionMs={player.state.positionMs}
                    durationMs={player.state.durationMs}
                    onSeek={(positionMs) => player.seek(positionMs)}
                    class="shrink-0"
                  />

                  <PlayerControls variant="full" class="shrink-0" />

                  <div class="flex shrink-0 items-center justify-around" data-no-swipe="true">
                    <IconButton
                      label={isFavorite(track()) ? 'Убрать из избранного' : 'В избранное'}
                      class={isFavorite(track()) ? 'text-accent' : undefined}
                      onClick={() => toggleFavorite(track())}
                    >
                      <Heart
                        fill={isFavorite(track()) ? 'currentColor' : 'none'}
                        aria-hidden="true"
                      />
                    </IconButton>

                    <IconButton label="Очередь" onClick={() => setQueueOpen(true)}>
                      <ListMusic aria-hidden="true" />
                    </IconButton>

                    <TrackMenu
                      track={track()}
                      triggerLabel="Действия с треком"
                      onNavigate={() => player.closeFull()}
                    />
                  </div>

                  <QueueSheet open={queueOpen()} onOpenChange={setQueueOpen} />
                </>
              )}
            </Show>
          </Dialog.Content>
        </div>
      </Dialog.Portal>
    </Dialog>
  )
}
