import { createEffect, createSignal, on, onCleanup } from 'solid-js'

import { cn, formatDuration } from '@/shared/lib'
import { Slider } from '@/shared/ui/Slider'

export type SeekBarProps = {
  positionMs: number
  durationMs: number
  disabled?: boolean
  onSeek: (positionMs: number) => void
  class?: string
}

/** Насколько близко должна подойти позиция из события, чтобы снять «залипание». */
const SETTLE_TOLERANCE_MS = 1500

/** Аварийный сброс, если событие с новой позицией так и не пришло. */
const SETTLE_TIMEOUT_MS = 2500

const STEP_MS = 1000

/**
 * Полоса перемотки. Во время перетаскивания входящие позиции игнорируются,
 * иначе ползунок дёргается от событий `player:state`.
 */
export function SeekBar(props: SeekBarProps) {
  const [dragValue, setDragValue] = createSignal<number | null>(null)
  const [seeked, setSeeked] = createSignal<number | null>(null)

  let settleTimer: ReturnType<typeof setTimeout> | null = null

  const stopSettleTimer = (): void => {
    if (settleTimer !== null) clearTimeout(settleTimer)
    settleTimer = null
  }

  onCleanup(stopSettleTimer)

  const duration = (): number => (props.durationMs > 0 ? props.durationMs : 0)

  const disabled = (): boolean => props.disabled === true || duration() === 0

  const value = (): number => {
    const dragged = dragValue()
    if (dragged !== null) return dragged

    const target = seeked()
    if (target !== null) return target

    return clamp(props.positionMs, 0, duration())
  }

  // Позиция из плеера догнала запрошенную — возвращаем управление событиям.
  createEffect(
    on(
      () => props.positionMs,
      (position) => {
        const target = seeked()
        if (target === null) return
        if (Math.abs(position - target) <= SETTLE_TOLERANCE_MS) {
          stopSettleTimer()
          setSeeked(null)
        }
      },
      { defer: true },
    ),
  )

  const commit = (next: number): void => {
    const target = Math.round(clamp(next, 0, duration()))
    setSeeked(target)
    setDragValue(null)

    stopSettleTimer()
    settleTimer = setTimeout(() => {
      settleTimer = null
      setSeeked(null)
    }, SETTLE_TIMEOUT_MS)

    props.onSeek(target)
  }

  return (
    <div class={cn('flex w-full flex-col gap-1', props.class)} data-no-swipe="true">
      <Slider
        value={value()}
        min={0}
        max={duration() === 0 ? STEP_MS : duration()}
        step={STEP_MS}
        disabled={disabled()}
        ariaLabel="Позиция воспроизведения"
        onChange={(next) => {
          setDragValue(next)
        }}
        onChangeEnd={commit}
      />

      <div class="flex items-center justify-between text-xs tabular-nums text-muted">
        <span>{formatDuration(value())}</span>
        <span>{formatDuration(duration())}</span>
      </div>
    </div>
  )
}

function clamp(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min
  return Math.min(Math.max(value, min), max)
}
