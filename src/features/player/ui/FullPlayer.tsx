import { Dialog } from '@kobalte/core/dialog'
import { ChevronDown, Heart, ListMusic, Music } from 'lucide-solid'
import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  on,
  onCleanup,
  onMount,
  Show,
} from 'solid-js'

import { glowGradient } from '@/shared/lib'
import { resolveArtwork } from '@/shared/ui/CoverArt'
import { EmptyState } from '@/shared/ui/EmptyState'
import { IconButton } from '@/shared/ui/IconButton'
import { clampDeckDrag, deckNeighbors, shouldCommitDeckSwipe } from '../model/deck'
import { isFavorite, toggleFavorite } from '../model/favorites'
import { createSwipe } from '../model/gestures'
import { usePlayer } from '../model/player-store'
import { ArtworkDeck } from './ArtworkDeck'
import { PlayerControls } from './PlayerControls'
import { QueueSheet } from './QueueSheet'
import { SeekBar } from './SeekBar'
import { TrackMenu } from './TrackMenu'

const DISMISS_THRESHOLD_PX = 90
const DECK_SETTLE_MS = 220
const TRACK_CHANGE_TIMEOUT_MS = 1200

/** Полноэкранный плеер: обложка, метаданные, перемотка, управление и очередь. */
export function FullPlayer() {
  const player = usePlayer()

  const [queueOpen, setQueueOpen] = createSignal(false)
  const [dragX, setDragX] = createSignal(0)
  const [dragY, setDragY] = createSignal(0)
  const [settling, setSettling] = createSignal(false)
  const [reducedMotion, setReducedMotion] = createSignal(false)

  let settleTimer: ReturnType<typeof setTimeout> | null = null
  let trackChangeTimer: ReturnType<typeof setTimeout> | null = null
  let returnFocus: HTMLElement | null = null

  const viewportWidth = (): number => window.innerWidth
  const neighbors = createMemo(() =>
    deckNeighbors(player.queue, player.state.queueIndex, player.state.repeat === 'ALL'),
  )

  /** Тянут влево — смотрим следующий трек, вправо — предыдущий. */
  const hasAdjacentFor = (dx: number): boolean =>
    (dx < 0 ? neighbors().next : neighbors().previous) !== null

  const clearTimer = (timer: ReturnType<typeof setTimeout> | null): void => {
    if (timer !== null) clearTimeout(timer)
  }

  const resetDeck = (): void => {
    clearTimer(settleTimer)
    clearTimer(trackChangeTimer)
    settleTimer = null
    trackChangeTimer = null
    setDragX(0)
    setDragY(0)
    setSettling(false)
  }

  const springToRest = (): void => {
    clearTimer(settleTimer)
    setSettling(true)
    setDragX(0)
    setDragY(0)
    settleTimer = setTimeout(() => {
      settleTimer = null
      setSettling(false)
    }, reducedMotion() ? 0 : DECK_SETTLE_MS)
  }

  const swipe = createSwipe({
    directions: ['left', 'right', 'down'],
    threshold: DISMISS_THRESHOLD_PX,
    onMove: ({ dx, dy }) => {
      if (dx !== 0) {
        setDragX(clampDeckDrag(dx, viewportWidth(), hasAdjacentFor(dx)))
        return
      }
      setDragY(dy > 0 ? dy : 0)
    },
    shouldCommit: (direction, end) => {
      if (direction === 'left' || direction === 'right') {
        const adjacent = direction === 'left' ? neighbors().next : neighbors().previous
        return shouldCommitDeckSwipe(end.dx, end.velocityX, viewportWidth(), adjacent !== null)
      }
      return direction === 'down' && end.dy >= DISMISS_THRESHOLD_PX
    },
    onSwipe: (direction) => {
      if (direction === 'down') {
        player.closeFull()
        return
      }

      const adjacent = direction === 'left' ? neighbors().next : neighbors().previous
      if (adjacent === null) {
        springToRest()
        return
      }

      clearTimer(settleTimer)
      setSettling(true)
      setDragY(0)
      setDragX(direction === 'left' ? -viewportWidth() : viewportWidth())

      const queueIndex = player.state.queueIndex
      if (direction === 'left') player.next()
      else player.prev()

      clearTimer(trackChangeTimer)
      trackChangeTimer = setTimeout(() => {
        trackChangeTimer = null
        if (player.state.queueIndex === queueIndex) resetDeck()
      }, TRACK_CHANGE_TIMEOUT_MS)
    },
    onEnd: springToRest,
  })

  createEffect(
    on(
      () => player.state.queueIndex,
      () => {
        resetDeck()
      },
      { defer: true },
    ),
  )

  createEffect(() => {
    if (!player.isFullOpen) resetDeck()
  })

  onMount(() => {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)')
    const sync = (): void => {
      setReducedMotion(query.matches)
    }
    sync()
    query.addEventListener('change', sync)
    onCleanup(() => query.removeEventListener('change', sync))
  })

  onCleanup(resetDeck)

  const subtitle = (): string => player.current?.artistName ?? 'Неизвестный исполнитель'

  const background = (): string => {
    const track = player.current
    if (track === null) return 'none'
    return glowGradient(`${track.albumTitle ?? track.title}·${track.artistName ?? ''}`)
  }

  /**
   * Фон плеера — сильно размытая обложка текущего трека (так делает VK Музыка).
   * Резолв идёт через тот же кэш, что и списки, поэтому лишнего запроса нет;
   * если обложки у трека нет, размывается glow-градиент-заглушка.
   */
  const [backdrop] = createResource(
    () => player.current?.coverKey ?? undefined,
    (coverKey: string) => resolveArtwork(coverKey),
  )

  const [backdropBroken, setBackdropBroken] = createSignal(false)

  // Не открылась картинка — не оставляем пустоту, уходим на градиент.
  createEffect(
    on(
      () => player.current?.coverKey,
      () => {
        setBackdropBroken(false)
      },
      { defer: true },
    ),
  )

  const backdropSource = (): string | undefined => {
    if (backdropBroken()) return undefined
    return backdrop() ?? undefined
  }

  return (
    <Dialog
      open={player.isFullOpen}
      onOpenChange={(open) => {
        if (!open) {
          resetDeck()
          player.closeFull()
        }
      }}
      modal
      preventScroll
    >
      <Dialog.Portal>
        <Dialog.Overlay class="modal__backdrop modal__backdrop--opaque h-dvh animate-in fade-in-0 duration-200" />

        <div class="fixed inset-0 z-50 modal__container h-dvh p-0 sm:w-full sm:p-0">
          <Dialog.Content
            class="modal__dialog modal__dialog--full safe-top safe-bottom animate-in slide-in-from-bottom relative isolate touch-none gap-4 overflow-hidden bg-background px-5 pt-2 pb-4 duration-300 ease-out-fluid md:mx-auto md:max-w-2xl"
            style={{ transform: `translateY(${String(dragY())}px)` }}
            onOpenAutoFocus={() => {
              returnFocus =
                document.activeElement instanceof HTMLElement ? document.activeElement : null
            }}
            onCloseAutoFocus={(event) => {
              event.preventDefault()
              returnFocus?.focus({ preventScroll: true })
              returnFocus = null
            }}
            onTouchStart={(event) => {
              if (!settling()) swipe.onTouchStart(event)
            }}
            onTouchMove={swipe.onTouchMove}
            onTouchEnd={swipe.onTouchEnd}
            onTouchCancel={swipe.onTouchCancel}
          >
            <div class="pointer-events-none absolute inset-0 z-0 overflow-hidden" aria-hidden="true">
              <Show
                when={backdropSource()}
                keyed
                fallback={
                  <div
                    class="absolute inset-0 scale-125 opacity-60 blur-[64px]"
                    style={{ 'background-image': background() }}
                  />
                }
              >
                {(source: string) => (
                  <img
                    src={source}
                    alt=""
                    class="animate-in fade-in-0 absolute inset-0 h-full w-full scale-125 object-cover opacity-70 blur-[64px] duration-700 ease-out"
                    decoding="async"
                    draggable={false}
                    onError={() => {
                      setBackdropBroken(true)
                    }}
                  />
                )}
              </Show>

              {/* Тексту и кнопкам нужен контраст: гасим фон сверху и снизу. */}
              <div class="absolute inset-0 bg-gradient-to-b from-background/70 via-background/25 to-background/88" />
            </div>

            <div class="relative z-10 flex min-h-0 flex-1 flex-col gap-4">
              <div class="flex items-center justify-between gap-2" data-no-swipe="true">
                <IconButton label="Свернуть плеер" onClick={() => player.closeFull()}>
                  <ChevronDown aria-hidden="true" />
                </IconButton>
                <span class="truncate text-xs tracking-wide text-muted uppercase">
                  Сейчас играет
                </span>
                <TrackMenu
                  track={player.current}
                  triggerLabel="Действия с треком"
                  onNavigate={() => player.closeFull()}
                />
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
                    <div class="flex min-h-0 flex-1 items-center">
                      <ArtworkDeck
                        previous={neighbors().previous}
                        current={neighbors().current ?? track()}
                        next={neighbors().next}
                        dragX={dragX()}
                        settling={settling()}
                        reducedMotion={reducedMotion()}
                      />
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

                    <div
                      class="flex shrink-0 items-center justify-around"
                      data-no-swipe="true"
                    >
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
                    </div>

                    <QueueSheet open={queueOpen()} onOpenChange={setQueueOpen} />
                  </>
                )}
              </Show>
            </div>
          </Dialog.Content>
        </div>
      </Dialog.Portal>
    </Dialog>
  )
}
