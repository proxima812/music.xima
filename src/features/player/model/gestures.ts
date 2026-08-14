/**
 * Тач-жесты плеера: свайпы по мини-плееру и закрытие полноэкранного вниз.
 * Никакой реактивности внутри — только вычисление дельты и порога.
 */

export type SwipeDirection = 'left' | 'right' | 'up' | 'down'

export type SwipeDelta = {
  dx: number
  dy: number
}

export type SwipeEnd = SwipeDelta & {
  velocityX: number
  velocityY: number
}

export type SwipeOptions = {
  /** Направления, на которые реагируем. */
  directions: readonly SwipeDirection[]
  onSwipe: (direction: SwipeDirection) => void
  /** Дистанция срабатывания, px. */
  threshold?: number
  /** Живая обратная связь во время жеста. */
  onMove?: (delta: SwipeDelta) => void
  /** Вызывается по завершении жеста, в том числе несостоявшегося. */
  onEnd?: () => void
  /** Решает, должен ли жест сработать по его конечным данным. */
  shouldCommit?: (direction: SwipeDirection, end: SwipeEnd) => boolean
}

export type SwipeHandlers = {
  onTouchStart: (event: TouchEvent) => void
  onTouchMove: (event: TouchEvent) => void
  onTouchEnd: (event: TouchEvent) => void
  onTouchCancel: (event: TouchEvent) => void
}

const DEFAULT_THRESHOLD = 60
const AXIS_LOCK_SLOP = 8
const AXIS_DOMINANCE = 1.15
const MAX_VELOCITY_SAMPLE_MS = 100

type SwipeAxis = 'pending' | 'horizontal' | 'vertical'

/** Элементы с `data-no-swipe` жест не начинают: слайдеры, списки, кнопки-переключатели. */
const IGNORE_SELECTOR = '[data-no-swipe]'

export function createSwipe(options: SwipeOptions): SwipeHandlers {
  let startX = 0
  let startY = 0
  let active = false
  let axis: SwipeAxis = 'pending'
  let lastMove: TouchSample | null = null

  const threshold = (): number => options.threshold ?? DEFAULT_THRESHOLD

  const allows = (direction: SwipeDirection): boolean => options.directions.includes(direction)

  const finish = (): void => {
    const wasActive = active
    active = false
    axis = 'pending'
    lastMove = null
    if (wasActive) options.onEnd?.()
  }

  return {
    onTouchStart: (event) => {
      const touch = event.touches[0]
      if (touch === undefined || event.touches.length > 1) {
        finish()
        return
      }
      if (isIgnored(event.target)) return

      startX = touch.clientX
      startY = touch.clientY
      axis = 'pending'
      lastMove = null
      active = true
    },

    onTouchMove: (event) => {
      if (!active) return
      const touch = event.touches[0]
      if (touch === undefined) return

      const dx = touch.clientX - startX
      const dy = touch.clientY - startY
      lastMove = toSample(touch, event.timeStamp)

      if (axis === 'pending') axis = resolveAxis(dx, dy)
      if (axis === 'pending') return

      options.onMove?.(axis === 'horizontal' ? { dx, dy: 0 } : { dx: 0, dy })
    },

    onTouchEnd: (event) => {
      if (!active) return

      const touch = event.changedTouches[0]
      if (touch === undefined) {
        finish()
        return
      }

      const dx = touch.clientX - startX
      const dy = touch.clientY - startY
      const resolvedAxis = axis === 'pending' ? resolveAxis(dx, dy) : axis
      const direction = resolveDirection(dx, dy, resolvedAxis)
      const end = { dx, dy, ...resolveVelocity(touch, event.timeStamp, lastMove) }
      const committed =
        direction !== null &&
        allows(direction) &&
        (options.shouldCommit?.(direction, end) ?? meetsThreshold(end, threshold(), resolvedAxis))

      finish()
      if (committed && direction !== null) options.onSwipe(direction)
    },

    onTouchCancel: () => {
      finish()
    },
  }
}

function resolveAxis(dx: number, dy: number): SwipeAxis {
  const absX = Math.abs(dx)
  const absY = Math.abs(dy)

  if (Math.max(absX, absY) < AXIS_LOCK_SLOP) return 'pending'
  if (absX >= absY * AXIS_DOMINANCE) return 'horizontal'
  if (absY >= absX * AXIS_DOMINANCE) return 'vertical'
  return 'pending'
}

function resolveDirection(dx: number, dy: number, axis: SwipeAxis): SwipeDirection | null {
  if (axis === 'horizontal') {
    return dx < 0 ? 'left' : 'right'
  }

  if (axis === 'vertical') return dy < 0 ? 'up' : 'down'
  return null
}

function meetsThreshold(end: SwipeEnd, threshold: number, axis: SwipeAxis): boolean {
  return axis === 'horizontal' ? Math.abs(end.dx) >= threshold : Math.abs(end.dy) >= threshold
}

type TouchSample = {
  timestamp: number
  x: number
  y: number
}

function toSample(touch: Touch, timestamp: number): TouchSample | null {
  if (!Number.isFinite(timestamp)) return null
  return { timestamp, x: touch.clientX, y: touch.clientY }
}

function resolveVelocity(
  touch: Touch,
  timestamp: number,
  lastMove: TouchSample | null,
): Pick<SwipeEnd, 'velocityX' | 'velocityY'> {
  if (lastMove === null || !Number.isFinite(timestamp)) return { velocityX: 0, velocityY: 0 }

  const elapsed = timestamp - lastMove.timestamp
  if (elapsed <= 0 || elapsed > MAX_VELOCITY_SAMPLE_MS) return { velocityX: 0, velocityY: 0 }

  return {
    velocityX: (touch.clientX - lastMove.x) / elapsed,
    velocityY: (touch.clientY - lastMove.y) / elapsed,
  }
}

function isIgnored(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest(IGNORE_SELECTOR) !== null
}
